package com.pumpkinmc.serverwrapper;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.net.wifi.WifiManager;
import android.os.Binder;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.PowerManager;
import android.util.Log;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class PumpkinService extends Service {
    public static final String ACTION_START = "com.pumpkinmc.serverwrapper.START";
    public static final String ACTION_STOP = "com.pumpkinmc.serverwrapper.STOP";
    public static final String ACTION_COMMAND = "com.pumpkinmc.serverwrapper.COMMAND";
    public static final String EXTRA_COMMAND = "command";

    private static final String CHANNEL_ID = "pumpkin_server";
    private static final String LOG_TAG = "PumpkinService";
    private static final int NOTIFICATION_ID = 25565;
    private static final int MAX_LOG_LINES = 600;

    private final IBinder binder = new LocalBinder();
    private final Object processLock = new Object();
    private final ArrayDeque<String> logLines = new ArrayDeque<>();
    private final ArrayList<PumpkinListener> listeners = new ArrayList<>();

    private Handler mainHandler;
    private ExecutorService executor;
    private Process process;
    private BufferedWriter commandWriter;
    private PowerManager.WakeLock wakeLock;
    private WifiManager.WifiLock wifiLock;
    private boolean running;

    public interface PumpkinListener {
        void onLogLine(String line);

        void onStateChanged(boolean isRunning);
    }

    public final class LocalBinder extends Binder {
        PumpkinService getService() {
            return PumpkinService.this;
        }
    }

    public static void startServer(Context context) {
        Intent intent = new Intent(context, PumpkinService.class).setAction(ACTION_START);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent);
        } else {
            context.startService(intent);
        }
    }

    public static void stopServer(Context context) {
        Intent intent = new Intent(context, PumpkinService.class).setAction(ACTION_STOP);
        context.startService(intent);
    }

    public static void sendCommand(Context context, String command) {
        Intent intent = new Intent(context, PumpkinService.class)
            .setAction(ACTION_COMMAND)
            .putExtra(EXTRA_COMMAND, command);
        context.startService(intent);
    }

    @Override
    public void onCreate() {
        super.onCreate();
        mainHandler = new Handler(Looper.getMainLooper());
        executor = Executors.newCachedThreadPool();
        createNotificationChannel();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return binder;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        String action = intent == null ? ACTION_START : intent.getAction();
        if (ACTION_STOP.equals(action)) {
            stopPumpkin();
            return START_NOT_STICKY;
        }
        if (ACTION_COMMAND.equals(action)) {
            String command = intent == null ? null : intent.getStringExtra(EXTRA_COMMAND);
            sendCommandToProcess(command);
            return running ? START_STICKY : START_NOT_STICKY;
        }

        startForegroundNotification("Starting server");
        startPumpkin();
        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        stopProcessImmediately();
        releaseLocks();
        if (executor != null) {
            executor.shutdownNow();
        }
        super.onDestroy();
    }

    public boolean isRunning() {
        synchronized (processLock) {
            return running;
        }
    }

    public List<String> getLogLines() {
        synchronized (logLines) {
            return new ArrayList<>(logLines);
        }
    }

    public void addListener(PumpkinListener listener) {
        listeners.add(listener);
        listener.onStateChanged(isRunning());
    }

    public void removeListener(PumpkinListener listener) {
        listeners.remove(listener);
    }

    public void sendConsoleCommand(String command) {
        sendCommandToProcess(command);
    }

    private void startPumpkin() {
        synchronized (processLock) {
            if (process != null && process.isAlive()) {
                appendLog("Pumpkin is already running.");
                updateForegroundNotification("Server running");
                return;
            }
        }

        executor.execute(() -> {
            File workDir = new File(getFilesDir(), "server");
            if (!workDir.exists() && !workDir.mkdirs()) {
                appendLog("Failed to create server directory: " + workDir);
                stopSelf();
                return;
            }

            try {
                copyAssetIfMissing("pumpkin.toml", new File(workDir, "pumpkin.toml"));
                installBundledCabbagePlugin(workDir);
                ProcessBuilder builder = new ProcessBuilder(getPumpkinExecutablePath());
                builder.directory(workDir);
                builder.redirectErrorStream(true);

                Map<String, String> environment = builder.environment();
                environment.put("RUST_BACKTRACE", "1");
                environment.put("NO_COLOR", "1");

                Process startedProcess = builder.start();
                BufferedWriter writer = new BufferedWriter(
                    new OutputStreamWriter(startedProcess.getOutputStream(), StandardCharsets.UTF_8)
                );

                synchronized (processLock) {
                    process = startedProcess;
                    commandWriter = writer;
                    running = true;
                }

                acquireLocks();
                appendLog("Started Pumpkin from " + getPumpkinExecutablePath());
                notifyStateChanged(true);
                updateForegroundNotification("Server running");

                executor.execute(() -> readProcessOutput(startedProcess));
                executor.execute(() -> waitForProcess(startedProcess));
            } catch (IOException | RuntimeException ex) {
                appendLog("Failed to start Pumpkin: " + ex.getMessage());
                notifyStateChanged(false);
                stopForegroundCompat();
                stopSelf();
            }
        });
    }

    private void stopPumpkin() {
        Process currentProcess;
        synchronized (processLock) {
            currentProcess = process;
        }

        if (currentProcess == null || !currentProcess.isAlive()) {
            appendLog("Pumpkin is not running.");
            clearProcessState(false);
            stopForegroundCompat();
            stopSelf();
            return;
        }

        appendLog("Stopping Pumpkin...");
        sendCommandToProcess("stop");
        updateForegroundNotification("Stopping server");
        mainHandler.postDelayed(() -> {
            synchronized (processLock) {
                if (process != null && process.isAlive()) {
                    appendLog("Pumpkin did not stop in time; forcing process exit.");
                    process.destroyForcibly();
                }
            }
        }, 12000L);
    }

    private void sendCommandToProcess(String command) {
        if (command == null || command.trim().isEmpty()) {
            return;
        }

        BufferedWriter writer;
        synchronized (processLock) {
            writer = commandWriter;
        }

        if (writer == null) {
            appendLog("Cannot send command; Pumpkin is not running.");
            return;
        }

        try {
            writer.write(command.trim());
            writer.newLine();
            writer.flush();
            appendLog("$ " + command.trim());
        } catch (IOException ex) {
            appendLog("Failed to send command: " + ex.getMessage());
        }
    }

    private void readProcessOutput(Process targetProcess) {
        try (BufferedReader reader = new BufferedReader(
            new InputStreamReader(targetProcess.getInputStream(), StandardCharsets.UTF_8)
        )) {
            String line;
            while ((line = reader.readLine()) != null) {
                appendLog(line);
            }
        } catch (IOException ex) {
            appendLog("Log reader stopped: " + ex.getMessage());
        }
    }

    private void waitForProcess(Process targetProcess) {
        try {
            int exitCode = targetProcess.waitFor();
            appendLog(String.format(Locale.US, "Pumpkin exited with code %d.", exitCode));
        } catch (InterruptedException ex) {
            Thread.currentThread().interrupt();
            appendLog("Pumpkin wait interrupted.");
        } finally {
            clearProcessState(true);
            releaseLocks();
            stopForegroundCompat();
            stopSelf();
        }
    }

    private void clearProcessState(boolean notify) {
        synchronized (processLock) {
            process = null;
            commandWriter = null;
            running = false;
        }
        if (notify) {
            notifyStateChanged(false);
        }
    }

    private void stopProcessImmediately() {
        synchronized (processLock) {
            if (process != null && process.isAlive()) {
                process.destroyForcibly();
            }
            process = null;
            commandWriter = null;
            running = false;
        }
    }

    private void copyAssetIfMissing(String assetName, File destination) throws IOException {
        if (destination.exists()) {
            return;
        }

        try (InputStream input = getAssets().open(assetName);
             FileOutputStream output = new FileOutputStream(destination)) {
            byte[] buffer = new byte[8192];
            int read;
            while ((read = input.read(buffer)) != -1) {
                output.write(buffer, 0, read);
            }
        }
    }

    private void installBundledCabbagePlugin(File workDir) throws IOException {
        File source = new File(getApplicationInfo().nativeLibraryDir, "libcabbage.so");
        if (!source.exists()) {
            appendLog("Bundled Cabbage plugin not found; starting without it.");
            return;
        }

        File pluginDir = new File(workDir, "plugins");
        if (!pluginDir.exists() && !pluginDir.mkdirs()) {
            throw new IOException("Failed to create plugin directory: " + pluginDir);
        }

        File destination = new File(pluginDir, "libcabbage.so");
        try (FileInputStream input = new FileInputStream(source);
             FileOutputStream output = new FileOutputStream(destination, false)) {
            byte[] buffer = new byte[8192];
            int read;
            while ((read = input.read(buffer)) != -1) {
                output.write(buffer, 0, read);
            }
        }

        appendLog("Staged bundled Cabbage plugin at " + destination.getAbsolutePath());
    }

    private String getPumpkinExecutablePath() {
        return getApplicationInfo().nativeLibraryDir + File.separator + "libpumpkin_exec.so";
    }

    private void appendLog(String line) {
        Log.i(LOG_TAG, line);
        mainHandler.post(() -> {
            synchronized (logLines) {
                logLines.addLast(line);
                while (logLines.size() > MAX_LOG_LINES) {
                    logLines.removeFirst();
                }
            }
            for (PumpkinListener listener : new ArrayList<>(listeners)) {
                listener.onLogLine(line);
            }
        });
    }

    private void notifyStateChanged(boolean isRunning) {
        mainHandler.post(() -> {
            for (PumpkinListener listener : new ArrayList<>(listeners)) {
                listener.onStateChanged(isRunning);
            }
        });
    }

    private void startForegroundNotification(String text) {
        Notification notification = buildNotification(text, true);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            );
        } else {
            startForeground(NOTIFICATION_ID, notification);
        }
    }

    private void updateForegroundNotification(String text) {
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.notify(NOTIFICATION_ID, buildNotification(text, true));
        }
    }

    private Notification buildNotification(String text, boolean ongoing) {
        Intent openIntent = new Intent(this, MainActivity.class);
        PendingIntent openPendingIntent = PendingIntent.getActivity(
            this,
            0,
            openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );

        Intent stopIntent = new Intent(this, PumpkinService.class).setAction(ACTION_STOP);
        PendingIntent stopPendingIntent = PendingIntent.getService(
            this,
            1,
            stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );

        Notification.Builder builder = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
            ? new Notification.Builder(this, CHANNEL_ID)
            : new Notification.Builder(this);

        builder
            .setSmallIcon(R.drawable.ic_stat_pumpkin)
            .setContentTitle("Pumpkin server")
            .setContentText(text)
            .setContentIntent(openPendingIntent)
            .setOngoing(ongoing)
            .setOnlyAlertOnce(true)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Stop", stopPendingIntent);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            builder.setForegroundServiceBehavior(Notification.FOREGROUND_SERVICE_IMMEDIATE);
        }

        return builder.build();
    }

    private void createNotificationChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            return;
        }
        NotificationChannel channel = new NotificationChannel(
            CHANNEL_ID,
            "Pumpkin server",
            NotificationManager.IMPORTANCE_LOW
        );
        channel.setDescription("Shows the foreground server status.");
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            manager.createNotificationChannel(channel);
        }
    }

    private void stopForegroundCompat() {
        mainHandler.post(() -> {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                stopForeground(STOP_FOREGROUND_REMOVE);
            } else {
                stopForeground(true);
            }
        });
    }

    private void acquireLocks() {
        mainHandler.post(() -> {
            try {
                PowerManager powerManager = (PowerManager) getSystemService(POWER_SERVICE);
                if (powerManager != null && (wakeLock == null || !wakeLock.isHeld())) {
                    wakeLock = powerManager.newWakeLock(
                        PowerManager.PARTIAL_WAKE_LOCK,
                        "Pumpkin:ServerWakeLock"
                    );
                    wakeLock.setReferenceCounted(false);
                    wakeLock.acquire();
                }
            } catch (RuntimeException ex) {
                appendLog("Could not acquire CPU wake lock: " + ex.getMessage());
            }

            try {
                WifiManager wifiManager = (WifiManager) getApplicationContext()
                    .getSystemService(WIFI_SERVICE);
                if (wifiManager != null && (wifiLock == null || !wifiLock.isHeld())) {
                    wifiLock = wifiManager.createWifiLock(
                        WifiManager.WIFI_MODE_FULL_HIGH_PERF,
                        "Pumpkin:ServerWifiLock"
                    );
                    wifiLock.setReferenceCounted(false);
                    wifiLock.acquire();
                }
            } catch (RuntimeException ex) {
                appendLog("Could not acquire Wi-Fi lock: " + ex.getMessage());
            }
        });
    }

    private void releaseLocks() {
        mainHandler.post(() -> {
            try {
                if (wifiLock != null && wifiLock.isHeld()) {
                    wifiLock.release();
                }
                wifiLock = null;
            } catch (RuntimeException ex) {
                appendLog("Could not release Wi-Fi lock: " + ex.getMessage());
            }

            try {
                if (wakeLock != null && wakeLock.isHeld()) {
                    wakeLock.release();
                }
                wakeLock = null;
            } catch (RuntimeException ex) {
                appendLog("Could not release CPU wake lock: " + ex.getMessage());
            }
        });
    }
}
