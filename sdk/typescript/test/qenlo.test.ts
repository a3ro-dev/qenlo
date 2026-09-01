import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { Collection, QenloError, type RecordInput } from "../src/index.js";

const records: readonly RecordInput[] = [
  { id: 9n, userId: 7n, timestamp: -5n, vector: [1, 0, 0] },
  { id: 2n, userId: 7n, timestamp: 0n, vector: [2, 0, 0] },
  { id: 4n, userId: 8n, timestamp: 10n, vector: [0, 1, 0] },
  { id: 6n, userId: 7n, timestamp: 20n, vector: [0, 0, 1] },
];

test("typed filters, deterministic ordering, and telemetry", () => {
  using db = Collection.memory(3);
  db.addBatch(records);
  const response = db.search([1, 0, 0], { userId: 7n, timestampLower: -5n, timestampUpper: 20n });
  assert.deepEqual(response.results.map((hit) => hit.id), [2n, 9n]);
  assert.equal(response.report.actualBackend, "Cpu");
  assert.equal(response.report.algorithm, "Exact");
  assert(response.report.operationId > 0n);
  assert.equal(db.stats().liveRows, 4);
});

test("atomic batches and non-reusable IDs", () => {
  using db = Collection.memory(3);
  db.add(records[0]!);
  assert.throws(() => db.addBatch([records[1]!, records[0]!]), QenloError);
  assert.equal(db.stats().rows, 1);
  db.delete(9n);
  assert.throws(() => db.add(records[0]!), QenloError);
});

test("durable reopen", () => {
  const root = mkdtempSync(join(tmpdir(), "qenlo-ts-"));
  const path = join(root, "vectors.qenlo");
  try {
    {
      using db = Collection.create(path, 3);
      db.addBatch(records);
      db.deleteBatch([2n, 4n]);
      db.flush();
    }
    using db = Collection.open(path, 3);
    assert.equal(db.stats().liveRows, 2);
    assert.deepEqual(db.search([1, 0, 0]).results.map((hit) => hit.id), [9n, 6n]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("validation and lifecycle failures are typed", () => {
  const db = Collection.memory(3);
  assert.throws(() => db.add({ id: 1n, userId: 1n, timestamp: 0n, vector: [1] }), RangeError);
  assert.throws(() => db.search([1, 0, 0], {}, 0), RangeError);
  assert.throws(() => db.add({ id: 1n, userId: 1n, timestamp: 0n, vector: [0, 0, 0] }), QenloError);
  db.close();
  db.close();
  assert.throws(() => db.stats(), QenloError);
});
