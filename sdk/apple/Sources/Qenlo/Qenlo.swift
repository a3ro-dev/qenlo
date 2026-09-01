import CQenlo
import Foundation

public struct QenloError: Error, Equatable, Sendable, CustomStringConvertible {
    public let message: String
    public var description: String { message }
    public init(_ message: String) { self.message = message }
}

public struct QenloRecord: Sendable, Equatable {
    public let id: UInt64
    public let userID: UInt64
    public let timestamp: Int64
    public let vector: [Float]
    public init(id: UInt64, userID: UInt64, timestamp: Int64, vector: [Float]) {
        self.id = id; self.userID = userID; self.timestamp = timestamp; self.vector = vector
    }
}

public struct QenloFilter: Sendable, Equatable {
    public let userID: UInt64?
    public let timestampLower: Int64?
    public let timestampUpper: Int64?
    public init(userID: UInt64? = nil, timestampLower: Int64? = nil, timestampUpper: Int64? = nil) {
        self.userID = userID; self.timestampLower = timestampLower; self.timestampUpper = timestampUpper
    }
}

public struct QenloSearchResult: Codable, Sendable, Equatable {
    public let id: UInt64
    public let distance: Float
}

public struct QenloExecutionReport: Sendable, Equatable {
    public let operationID: UInt64
    public let requestedBackend: String
    public let actualBackend: String
    public let algorithm: String
    public let filterExecution: String
    public let indexGeneration: UInt64
    public let rebuilt: Bool
    public let routingReason: String?
    public let fallbackReason: String?
    public let totalDurationNanoseconds: UInt64
    public let lockWaitNanoseconds: UInt64
    public let eligibleRows: UInt64?
    public let uploadBytes: UInt64?
    public let readbackBytes: UInt64?
    public let allocationBytes: UInt64?
    public let dispatchCount: UInt32?
    public let candidates: UInt64?
    public let batchSize: Int
}

public struct QenloSearchResponse: Sendable, Equatable {
    public let results: [QenloSearchResult]
    public let report: QenloExecutionReport
}

public struct QenloCollectionStats: Sendable, Equatable {
    public let dimension: Int
    public let rows: Int
    public let liveRows: Int
    public let generation: UInt64
    public let preparedGeneration: UInt64?
    public let durableGeneration: UInt64?
    public let recoveredInterruptedWrite: Bool
    public let closed: Bool
}

private struct WireHit: Decodable { let id: String; let distance: Float }
private struct WireReport: Decodable {
    let operationID, indexGeneration, totalDurationNanoseconds, lockWaitNanoseconds: String
    let requestedBackend, actualBackend, algorithm, filterExecution: String
    let rebuilt: Bool
    let routingReason, fallbackReason, eligibleRows, uploadBytes, readbackBytes, allocationBytes, candidates: String?
    let dispatchCount: UInt32?
    let batchSize: Int
    enum CodingKeys: String, CodingKey {
        case operationID = "operation_id", requestedBackend = "requested_backend", actualBackend = "actual_backend", algorithm
        case filterExecution = "filter_execution", indexGeneration = "index_generation", rebuilt
        case routingReason = "routing_reason", fallbackReason = "fallback_reason"
        case totalDurationNanoseconds = "total_duration_ns", lockWaitNanoseconds = "lock_wait_ns"
        case eligibleRows = "eligible_rows", uploadBytes = "upload_bytes", readbackBytes = "readback_bytes"
        case allocationBytes = "allocation_bytes", dispatchCount = "dispatch_count", candidates, batchSize = "batch_size"
    }
}
private struct WireSearch: Decodable { let results: [WireHit]; let report: WireReport }
private struct WireStats: Decodable {
    let dimension, rows, liveRows: Int
    let generation: String
    let preparedGeneration, durableGeneration: String?
    let recoveredInterruptedWrite, closed: Bool
    enum CodingKeys: String, CodingKey {
        case dimension, rows, liveRows = "live_rows", generation, preparedGeneration = "prepared_generation"
        case durableGeneration = "durable_generation", recoveredInterruptedWrite = "recovered_interrupted_write", closed
    }
}

public final class QenloCollection: @unchecked Sendable {
    public let dimension: Int
    private let lock = NSRecursiveLock()
    private var handle: OpaquePointer?

    public init(memoryDimension dimension: Int) throws {
        guard dimension > 0 else { throw QenloError("dimension must be positive") }
        self.dimension = dimension
        self.handle = qenlo_collection_new(dimension)
        guard handle != nil else { throw QenloError(Self.lastError()) }
    }

    public init(create path: URL, dimension: Int) throws {
        guard dimension > 0 else { throw QenloError("dimension must be positive") }
        self.dimension = dimension
        self.handle = path.path.withCString { qenlo_collection_create($0, dimension) }
        guard handle != nil else { throw QenloError(Self.lastError()) }
    }

    public init(open path: URL, dimension: Int) throws {
        guard dimension > 0 else { throw QenloError("dimension must be positive") }
        self.dimension = dimension
        self.handle = path.path.withCString { qenlo_collection_open($0, dimension) }
        guard handle != nil else { throw QenloError(Self.lastError()) }
    }

    deinit { try? close() }

    private static func take(_ pointer: UnsafeMutablePointer<CChar>?) throws -> String {
        guard let pointer else { throw QenloError(lastError()) }
        defer { qenlo_string_free(pointer) }
        return String(cString: pointer)
    }

    private static func lastError() -> String {
        guard let pointer = qenlo_last_error() else { return "unknown Qenlo native error" }
        defer { qenlo_string_free(pointer) }
        return String(cString: pointer)
    }

    private func withHandle<T>(_ body: (OpaquePointer) throws -> T) throws -> T {
        lock.lock(); defer { lock.unlock() }
        guard let handle else { throw QenloError("collection is closed") }
        return try body(handle)
    }

    private func checked(_ status: Int32) throws { if status != 0 { throw QenloError(Self.lastError()) } }
    private func checkedVector(_ vector: [Float]) throws -> [Float] {
        guard vector.count == dimension else { throw QenloError("expected vector dimension \(dimension), got \(vector.count)") }
        return vector
    }

    public func add(_ record: QenloRecord) throws {
        let vector = try checkedVector(record.vector)
        try withHandle { handle in try vector.withUnsafeBufferPointer { try checked(qenlo_add(handle, record.id, record.userID, record.timestamp, $0.baseAddress, vector.count)) } }
    }

    public func addBatch(_ records: [QenloRecord]) throws {
        guard !records.isEmpty else { return }
        let ids = records.map(\.id), users = records.map(\.userID), timestamps = records.map(\.timestamp)
        let vectors = try records.flatMap { try checkedVector($0.vector) }
        try withHandle { handle in
            try ids.withUnsafeBufferPointer { ids in try users.withUnsafeBufferPointer { users in try timestamps.withUnsafeBufferPointer { timestamps in try vectors.withUnsafeBufferPointer { vectors in
                try checked(qenlo_add_batch(handle, ids.baseAddress, users.baseAddress, timestamps.baseAddress, vectors.baseAddress, records.count, dimension))
            } } } }
        }
    }

    public func delete(_ id: UInt64) throws { try withHandle { try checked(qenlo_delete($0, id)) } }
    public func deleteBatch(_ ids: [UInt64]) throws {
        guard !ids.isEmpty else { return }
        try withHandle { handle in try ids.withUnsafeBufferPointer { try checked(qenlo_delete_batch(handle, $0.baseAddress, ids.count)) } }
    }

    public func search(_ query: [Float], filter: QenloFilter = .init(), k: Int = 10) throws -> QenloSearchResponse {
        guard (1...64).contains(k) else { throw QenloError("k must be in 1...64") }
        let query = try checkedVector(query)
        return try withHandle { handle in
            let json = try query.withUnsafeBufferPointer { values in
                try Self.take(qenlo_search(handle, values.baseAddress, query.count, filter.userID != nil, filter.userID ?? 0, filter.timestampLower != nil, filter.timestampLower ?? 0, filter.timestampUpper != nil, filter.timestampUpper ?? 0, k))
            }
            let wire = try JSONDecoder().decode(WireSearch.self, from: Data(json.utf8))
            guard let operation = UInt64(wire.report.operationID), let generation = UInt64(wire.report.indexGeneration), let total = UInt64(wire.report.totalDurationNanoseconds), let wait = UInt64(wire.report.lockWaitNanoseconds) else { throw QenloError("invalid 64-bit telemetry from native library") }
            return QenloSearchResponse(
                results: try wire.results.map { guard let id = UInt64($0.id) else { throw QenloError("invalid result ID") }; return .init(id: id, distance: $0.distance) },
                report: .init(operationID: operation, requestedBackend: wire.report.requestedBackend, actualBackend: wire.report.actualBackend, algorithm: wire.report.algorithm, filterExecution: wire.report.filterExecution, indexGeneration: generation, rebuilt: wire.report.rebuilt, routingReason: wire.report.routingReason, fallbackReason: wire.report.fallbackReason, totalDurationNanoseconds: total, lockWaitNanoseconds: wait, eligibleRows: wire.report.eligibleRows.flatMap(UInt64.init), uploadBytes: wire.report.uploadBytes.flatMap(UInt64.init), readbackBytes: wire.report.readbackBytes.flatMap(UInt64.init), allocationBytes: wire.report.allocationBytes.flatMap(UInt64.init), dispatchCount: wire.report.dispatchCount, candidates: wire.report.candidates.flatMap(UInt64.init), batchSize: wire.report.batchSize)
            )
        }
    }

    public func stats() throws -> QenloCollectionStats {
        try withHandle { handle in
            let wire = try JSONDecoder().decode(WireStats.self, from: Data(try Self.take(qenlo_stats(handle)).utf8))
            guard let generation = UInt64(wire.generation) else { throw QenloError("invalid generation") }
            return .init(dimension: wire.dimension, rows: wire.rows, liveRows: wire.liveRows, generation: generation, preparedGeneration: wire.preparedGeneration.flatMap(UInt64.init), durableGeneration: wire.durableGeneration.flatMap(UInt64.init), recoveredInterruptedWrite: wire.recoveredInterruptedWrite, closed: wire.closed)
        }
    }

    public func flush() throws { try withHandle { try checked(qenlo_flush($0)) } }

    public func close() throws {
        lock.lock(); defer { lock.unlock() }
        guard let value = handle else { return }
        handle = nil
        let status = qenlo_close(value)
        qenlo_collection_free(value)
        try checked(status)
    }
}
