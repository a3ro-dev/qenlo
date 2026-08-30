package dev.qenlo.lab;

import android.app.*;
import android.os.*;
import android.content.*;
import android.graphics.Color;
import android.net.Uri;
import android.view.*;
import android.widget.*;
import org.json.JSONObject;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import java.util.concurrent.*;

public final class MainActivity extends Activity {
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private Button run, upload, github;
    private ProgressBar progress;
    private TextView status, result;
    private Spinner profile;
    private EditText endpoint, token;
    private String report;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(buildUi());
        File saved = new File(getFilesDir(), "last-run.json");
        if (saved.isFile()) try { report = read(saved); showSummary(); } catch (Exception ignored) {}
    }

    private View buildUi() {
        int pad = dp(20);
        LinearLayout body = new LinearLayout(this); body.setOrientation(LinearLayout.VERTICAL); body.setPadding(pad,pad,pad,pad); body.setBackgroundColor(Color.rgb(247,245,240));
        TextView title = text("Qenlo device lab", 28, true); body.addView(title);
        body.addView(text(device(), 13, false));
        profile = new Spinner(this); profile.setAdapter(new ArrayAdapter<>(this, android.R.layout.simple_spinner_dropdown_item, new String[]{"quick","full","soak"})); body.addView(profile);
        run = button("Run local suite"); body.addView(run); run.setOnClickListener(v -> runSuite());
        progress = new ProgressBar(this); progress.setVisibility(View.GONE); body.addView(progress);
        status = text("Ready. Keep the phone connected to power for full or soak runs.", 14, false); body.addView(status);
        result = text("No retained result yet.", 13, false); result.setTextIsSelectable(true); body.addView(result);
        github = button("Copy report and open GitHub"); github.setEnabled(false); body.addView(github); github.setOnClickListener(v -> openGitHub());
        endpoint = input("https://your-lab.example/api/v1/runs", false); body.addView(endpoint);
        token = input("Bearer token", true); body.addView(token);
        upload = button("Submit retained result"); upload.setEnabled(false); body.addView(upload); upload.setOnClickListener(v -> upload());
        TextView privacy = text("Telemetry contains device class, OS, aggregate latency, recall, routing, and failures. It never includes vectors or source data.", 12, false); body.addView(privacy);
        ScrollView scroll = new ScrollView(this); scroll.addView(body); return scroll;
    }

    private void runSuite() {
        setBusy(true, "Running native suite. Do not background the app.");
        String selected = profile.getSelectedItem().toString();
        worker.execute(() -> {
            String raw = NativeLab.run(selected);
            try {
                JSONObject json = new JSONObject(raw);
                if (json.has("bridge_error")) throw new IOException(json.getString("bridge_error"));
                json.put("install_id", installId()); json.put("target", "android-arm64");
                json.put("os", "android"); json.put("os_version", Build.VERSION.RELEASE + " (API " + Build.VERSION.SDK_INT + ")");
                json.put("cpu_arch", Build.SUPPORTED_ABIS.length == 0 ? "unknown" : Build.SUPPORTED_ABIS[0]);
                json.put("cpu_name", soc());
                PowerManager power = getSystemService(PowerManager.class);
                if (Build.VERSION.SDK_INT >= 29) json.put("thermal_state", Integer.toString(power.getCurrentThermalStatus()));
                report = json.toString(); write(new File(getFilesDir(), "last-run.json"), report);
                runOnUiThread(() -> { setBusy(false, "Suite complete. Result retained locally."); showSummary(); });
            } catch (Exception error) { runOnUiThread(() -> setBusy(false, "Suite failed: " + error.getMessage())); }
        });
    }

    private void upload() {
        if (report == null) return;
        String url = endpoint.getText().toString().trim(), secret = token.getText().toString();
        if (!url.startsWith("https://")) { status.setText("Submission requires an HTTPS endpoint."); return; }
        setBusy(true, "Submitting retained result…");
        worker.execute(() -> {
            try {
                HttpURLConnection connection = (HttpURLConnection)new URL(url).openConnection();
                connection.setConnectTimeout(15000); connection.setReadTimeout(30000); connection.setInstanceFollowRedirects(false);
                connection.setRequestMethod("POST"); connection.setRequestProperty("Authorization", "Bearer " + secret); connection.setRequestProperty("Content-Type", "application/json"); connection.setDoOutput(true);
                try(OutputStream out=connection.getOutputStream()){out.write(report.getBytes(StandardCharsets.UTF_8));}
                int code=connection.getResponseCode(); if(code<200||code>=300) throw new IOException("server returned HTTP "+code);
                runOnUiThread(() -> setBusy(false, "Submitted. The local result remains on this device."));
            } catch(Exception error){runOnUiThread(() -> setBusy(false, "Submission failed; local result retained: "+error.getMessage()));}
        });
    }

    private void openGitHub() {
        if (report == null) return;
        ClipboardManager clipboard = (ClipboardManager)getSystemService(CLIPBOARD_SERVICE);
        clipboard.setPrimaryClip(ClipData.newPlainText("Qenlo device lab report", report));
        startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/a3ro-dev/qenlo/issues/new?template=device-lab-report.yml")));
        status.setText("Report copied. Paste it into the GitHub report field and submit.");
    }

    private void showSummary() { try { JSONObject json=new JSONObject(report); result.setText("run "+json.getString("run_id")+"\n"+json.getJSONArray("cells").length()+" workload cells · "+json.getJSONArray("failures").length()+" failures"); upload.setEnabled(true); github.setEnabled(true); } catch(Exception e){result.setText("Retained result is unreadable.");} }
    private void setBusy(boolean busy,String message){run.setEnabled(!busy);upload.setEnabled(!busy&&report!=null);github.setEnabled(!busy&&report!=null);progress.setVisibility(busy?View.VISIBLE:View.GONE);status.setText(message);}
    private String soc(){return Build.VERSION.SDK_INT>=31?Build.SOC_MANUFACTURER+" "+Build.SOC_MODEL:Build.MANUFACTURER+" "+Build.HARDWARE;}
    private String device(){return Build.MANUFACTURER+" "+Build.MODEL+"\n"+soc()+" · "+(Build.SUPPORTED_ABIS.length==0?"unknown":Build.SUPPORTED_ABIS[0]);}
    private String installId(){android.content.SharedPreferences p=getSharedPreferences("lab",MODE_PRIVATE);String id=p.getString("install_id",null);if(id==null){id=UUID.randomUUID().toString();p.edit().putString("install_id",id).apply();}return id;}
    private TextView text(String value,int sp,boolean strong){TextView v=new TextView(this);v.setText(value);v.setTextSize(sp);v.setTextColor(Color.rgb(29,35,32));v.setPadding(0,dp(8),0,dp(8));if(strong)v.setTypeface(v.getTypeface(),1);return v;}
    private Button button(String value){Button v=new Button(this);v.setText(value);v.setAllCaps(false);return v;}
    private EditText input(String hint,boolean password){EditText v=new EditText(this);v.setHint(hint);v.setSingleLine(true);if(password)v.setInputType(0x81);return v;}
    private int dp(int value){return Math.round(value*getResources().getDisplayMetrics().density);}
    private static String read(File file)throws IOException{return new String(java.nio.file.Files.readAllBytes(file.toPath()),StandardCharsets.UTF_8);}
    private static void write(File file,String value)throws IOException{try(FileOutputStream out=new FileOutputStream(file)){out.write(value.getBytes(StandardCharsets.UTF_8));out.getFD().sync();}}
    @Override protected void onDestroy(){super.onDestroy();if(isFinishing())worker.shutdown();}
}
