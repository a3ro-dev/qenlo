import Foundation
import Testing
@testable import Qenlo

private let records = [
    QenloRecord(id: 9, userID: 7, timestamp: -5, vector: [1, 0, 0]),
    QenloRecord(id: 2, userID: 7, timestamp: 0, vector: [2, 0, 0]),
    QenloRecord(id: 4, userID: 8, timestamp: 10, vector: [0, 1, 0]),
    QenloRecord(id: 6, userID: 7, timestamp: 20, vector: [0, 0, 1]),
]

@Test func typedFilterOrderingAndTelemetry() throws {
    let db = try QenloCollection(memoryDimension: 3); defer { try? db.close() }
    try db.addBatch(records)
    let response = try db.search([1, 0, 0], filter: .init(userID: 7, timestampLower: -5, timestampUpper: 20))
    #expect(response.results.map(\.id) == [2, 9])
    #expect(response.report.actualBackend == "Cpu")
    #expect(response.report.algorithm == "Exact")
    #expect(response.report.operationID > 0)
}

@Test func atomicBatchAndNonReusableIDs() throws {
    let db = try QenloCollection(memoryDimension: 3); defer { try? db.close() }
    try db.add(records[0])
    #expect(throws: QenloError.self) { try db.addBatch([records[1], records[0]]) }
    #expect(try db.stats().rows == 1)
    try db.delete(9)
    #expect(throws: QenloError.self) { try db.add(records[0]) }
}

@Test func durableReopen() throws {
    let path = FileManager.default.temporaryDirectory.appending(path: "qenlo-swift-\(UUID())")
    defer { try? FileManager.default.removeItem(at: path) }
    let created = try QenloCollection(create: path, dimension: 3)
    try created.addBatch(records); try created.deleteBatch([2, 4]); try created.flush(); try created.close()
    let opened = try QenloCollection(open: path, dimension: 3); defer { try? opened.close() }
    #expect(try opened.stats().liveRows == 2)
}

@Test func portableQNRoundTrip() throws {
    let path = FileManager.default.temporaryDirectory.appending(path: "qenlo-swift-\(UUID()).qn")
    defer { try? FileManager.default.removeItem(at: path) }
    let db = try QenloCollection(memoryDimension: 3)
    try db.addBatch(records); try db.delete(9); try db.exportQN(to: path)
    #expect(throws: QenloError.self) { try db.exportQN(to: path) }
    try db.close()
    let imported = try QenloCollection(importQN: path, dimension: 3); defer { try? imported.close() }
    #expect(try imported.stats().generation == 5)
    #expect(try imported.stats().liveRows == 3)
}

@Test func validationAndLifecycle() throws {
    let db = try QenloCollection(memoryDimension: 3)
    #expect(throws: QenloError.self) { try db.add(.init(id: 1, userID: 1, timestamp: 0, vector: [1])) }
    #expect(throws: QenloError.self) { try db.search([1, 0, 0], k: 0) }
    #expect(throws: QenloError.self) { try db.add(.init(id: 1, userID: 1, timestamp: 0, vector: [0, 0, 0])) }
    try db.close(); try db.close()
    #expect(throws: QenloError.self) { try db.stats() }
}
