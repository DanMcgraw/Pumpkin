package com.pumpkinmc.serverwrapper;

import android.Manifest;
import android.app.Activity;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.text.InputType;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.List;

public final class MainActivity extends Activity {
    public static final String EXTRA_START_SERVER = "start_server";

    private PumpkinService service;
    private boolean bound;
    private TextView statusText;
    private TextView consoleText;
    private EditText commandInput;
    private ScrollView consoleScroll;

    private final PumpkinService.PumpkinListener listener = new PumpkinService.PumpkinListener() {
        @Override
        public void onLogLine(String line) {
            appendConsoleLine(line);
        }

        @Override
        public void onStateChanged(boolean isRunning) {
            updateStatus(isRunning);
        }
    };

    private final ServiceConnection connection = new ServiceConnection() {
        @Override
        public void onServiceConnected(ComponentName name, IBinder binder) {
            PumpkinService.LocalBinder localBinder = (PumpkinService.LocalBinder) binder;
            service = localBinder.getService();
            bound = true;
            service.addListener(listener);
            renderConsole(service.getLogLines());
            updateStatus(service.isRunning());
        }

        @Override
        public void onServiceDisconnected(ComponentName name) {
            if (service != null) {
                service.removeListener(listener);
            }
            bound = false;
            service = null;
            updateStatus(false);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        requestNotificationPermission();
        setContentView(buildContentView());
        handleIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        handleIntent(intent);
    }

    @Override
    protected void onStart() {
        super.onStart();
        bindService(
            new Intent(this, PumpkinService.class),
            connection,
            Context.BIND_AUTO_CREATE
        );
    }

    @Override
    protected void onStop() {
        if (bound) {
            if (service != null) {
                service.removeListener(listener);
            }
            unbindService(connection);
            bound = false;
            service = null;
        }
        super.onStop();
    }

    private LinearLayout buildContentView() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(16), dp(16), dp(16), dp(16));
        root.setBackgroundColor(Color.rgb(17, 22, 20));

        TextView title = new TextView(this);
        title.setText("Pumpkin Server");
        title.setTextColor(Color.WHITE);
        title.setTextSize(24);
        title.setTypeface(Typeface.DEFAULT_BOLD);
        root.addView(title, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ));

        statusText = new TextView(this);
        statusText.setTextColor(Color.rgb(175, 190, 182));
        statusText.setTextSize(14);
        statusText.setPadding(0, dp(6), 0, dp(14));
        root.addView(statusText, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ));

        LinearLayout controls = new LinearLayout(this);
        controls.setOrientation(LinearLayout.HORIZONTAL);
        controls.setGravity(Gravity.CENTER_VERTICAL);

        Button startButton = new Button(this);
        startButton.setText("Start");
        startButton.setOnClickListener(view -> PumpkinService.startServer(this));
        controls.addView(startButton, new LinearLayout.LayoutParams(
            0,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            1
        ));

        Button stopButton = new Button(this);
        stopButton.setText("Stop");
        stopButton.setOnClickListener(view -> PumpkinService.stopServer(this));
        LinearLayout.LayoutParams stopParams = new LinearLayout.LayoutParams(
            0,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            1
        );
        stopParams.setMargins(dp(10), 0, 0, 0);
        controls.addView(stopButton, stopParams);
        root.addView(controls, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ));

        LinearLayout commandRow = new LinearLayout(this);
        commandRow.setOrientation(LinearLayout.HORIZONTAL);
        commandRow.setPadding(0, dp(12), 0, dp(12));

        commandInput = new EditText(this);
        commandInput.setSingleLine(true);
        commandInput.setHint("Command");
        commandInput.setTextColor(Color.WHITE);
        commandInput.setHintTextColor(Color.rgb(130, 145, 136));
        commandInput.setInputType(InputType.TYPE_CLASS_TEXT);
        commandRow.addView(commandInput, new LinearLayout.LayoutParams(
            0,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            1
        ));

        Button sendButton = new Button(this);
        sendButton.setText("Send");
        sendButton.setOnClickListener(view -> sendCurrentCommand());
        LinearLayout.LayoutParams sendParams = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        );
        sendParams.setMargins(dp(10), 0, 0, 0);
        commandRow.addView(sendButton, sendParams);
        root.addView(commandRow, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ));

        consoleText = new TextView(this);
        consoleText.setTextColor(Color.rgb(222, 231, 225));
        consoleText.setTextSize(12);
        consoleText.setTypeface(Typeface.MONOSPACE);
        consoleText.setTextIsSelectable(true);
        consoleText.setPadding(dp(10), dp(10), dp(10), dp(10));
        consoleText.setBackgroundColor(Color.rgb(6, 9, 8));

        consoleScroll = new ScrollView(this);
        consoleScroll.addView(consoleText, new ScrollView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ));
        root.addView(consoleScroll, new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            0,
            1
        ));

        updateStatus(false);
        return root;
    }

    private void sendCurrentCommand() {
        String command = commandInput.getText().toString().trim();
        if (command.isEmpty()) {
            return;
        }
        if (bound && service != null) {
            service.sendConsoleCommand(command);
        } else {
            PumpkinService.sendCommand(this, command);
        }
        commandInput.setText("");
    }

    private void renderConsole(List<String> lines) {
        StringBuilder builder = new StringBuilder();
        for (String line : lines) {
            builder.append(line).append('\n');
        }
        consoleText.setText(builder.toString());
        scrollConsoleToBottom();
    }

    private void appendConsoleLine(String line) {
        consoleText.append(line);
        consoleText.append("\n");
        scrollConsoleToBottom();
    }

    private void scrollConsoleToBottom() {
        consoleScroll.post(() -> consoleScroll.fullScroll(ScrollView.FOCUS_DOWN));
    }

    private void updateStatus(boolean isRunning) {
        statusText.setText(isRunning
            ? "Running in a foreground service"
            : "Stopped");
    }

    private void requestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU
            && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
            != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { Manifest.permission.POST_NOTIFICATIONS }, 100);
        }
    }

    private void handleIntent(Intent intent) {
        if (intent != null && intent.getBooleanExtra(EXTRA_START_SERVER, false)) {
            PumpkinService.startServer(this);
        }
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
