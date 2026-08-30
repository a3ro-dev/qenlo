import SwiftUI
import UIKit

@MainActor final class LabModel: ObservableObject {
    @Published var profile = "quick"
    @Published var endpoint = ""
    @Published var token = ""
    @Published var status = "Ready. Connect power for full or soak runs."
    @Published var summary = "No retained result yet."
    @Published var busy = false
    private(set) var report: Data?

    init() { loadRetained() }

    func run() {
        busy = true; status = "Running native suite. Keep Qenlo Lab in the foreground."
        let selected = profile
        let work = Task.detached(priority: .userInitiated) { () throws -> Data in
            guard let pointer = selected.withCString({ qenlo_lab_run($0) }) else { throw LabError.bridge("native runner returned null") }
            defer { qenlo_lab_free(pointer) }
            guard let raw = String(validatingUTF8: pointer), var json = try JSONSerialization.jsonObject(with: Data(raw.utf8)) as? [String: Any] else { throw LabError.bridge("native runner returned invalid JSON") }
            if let error = json["bridge_error"] as? String { throw LabError.bridge(error) }
            json["install_id"] = await Self.installID()
            json["target"] = "ios-arm64"; json["os"] = "ios"
            json["os_version"] = await UIDevice.current.systemVersion
            json["cpu_arch"] = "arm64"; json["cpu_name"] = Self.machine()
            json["thermal_state"] = String(ProcessInfo.processInfo.thermalState.rawValue)
            let data = try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys])
            try FileManager.default.createDirectory(at: Self.reportURL.deletingLastPathComponent(), withIntermediateDirectories: true)
            try data.write(to: Self.reportURL, options: .atomic)
            return data
        }
        Task {
            do { accept(try await work.value) }
            catch { busy = false; status = "Suite failed: \(error.localizedDescription)" }
        }
    }

    func submit() {
        guard let report, let url = URL(string: endpoint), url.scheme == "https", url.host != nil, url.user == nil, url.password == nil else { status = "Submission requires a valid HTTPS endpoint without embedded credentials."; return }
        busy = true; status = "Submitting retained result…"
        Task {
            do {
                var request = URLRequest(url: url); request.httpMethod = "POST"; request.timeoutInterval = 30
                request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization"); request.setValue("application/json", forHTTPHeaderField: "Content-Type"); request.httpBody = report
                let session = URLSession(configuration: .ephemeral, delegate: NoRedirectDelegate(), delegateQueue: nil)
                defer { session.finishTasksAndInvalidate() }
                let (_, response) = try await session.data(for: request)
                guard let http = response as? HTTPURLResponse, 200..<300 ~= http.statusCode else { throw LabError.bridge("server rejected the result") }
                status = "Submitted. The local result remains on this iPhone."
            } catch { status = "Submission failed; local result retained: \(error.localizedDescription)" }
            busy = false
        }
    }

    private func accept(_ data: Data) {
        report = data; busy = false; status = "Suite complete. Result retained locally."
        if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any], let cells = json["cells"] as? [Any], let failures = json["failures"] as? [Any] { summary = "\(cells.count) workload cells · \(failures.count) failures" }
    }
    private func loadRetained() { if let data = try? Data(contentsOf: Self.reportURL) { accept(data) } }
    private static var reportURL: URL { FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("qenlo-last-run.json") }
    private static func installID() -> String { let key="qenlo.install-id"; if let id=UserDefaults.standard.string(forKey:key){return id};let id=UUID().uuidString;UserDefaults.standard.set(id,forKey:key);return id }
    nonisolated static func machine() -> String { var size=0;sysctlbyname("hw.machine",nil,&size,nil,0);var value=[CChar](repeating:0,count:size);sysctlbyname("hw.machine",&value,&size,nil,0);return String(cString:value) }
}

enum LabError: LocalizedError { case bridge(String); var errorDescription: String? { if case .bridge(let value)=self{return value};return nil } }

final class NoRedirectDelegate: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
}

struct LabView: View {
    @StateObject private var model = LabModel()
    var body: some View {
        NavigationStack {
            Form {
                Section("Device") { LabeledContent("model", value: LabModel.machine()); LabeledContent("system", value: UIDevice.current.systemVersion); LabeledContent("thermal", value: String(ProcessInfo.processInfo.thermalState.rawValue)) }
                Section("Suite") { Picker("profile", selection: $model.profile) { Text("quick").tag("quick"); Text("full").tag("full"); Text("soak").tag("soak") }.pickerStyle(.segmented); Button("Run local suite", action: model.run).disabled(model.busy); if model.busy { ProgressView() }; Text(model.status).foregroundStyle(.secondary) }
                Section("Retained result") { Text(model.summary).font(.system(.body, design: .monospaced)).textSelection(.enabled) }
                Section("Submit") { TextField("https://lab.example/api/v1/runs", text: $model.endpoint).textInputAutocapitalization(.never).keyboardType(.URL); SecureField("Bearer token", text: $model.token); Button("Submit retained result", action: model.submit).disabled(model.busy || model.report == nil); Text("Telemetry contains device class and aggregate test measurements. It never contains vectors or source data.").font(.footnote).foregroundStyle(.secondary) }
            }.navigationTitle("Qenlo device lab")
        }.tint(Color(red: 0.71, green: 0.24, blue: 0.18))
    }
}
