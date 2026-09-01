/** Type-safe Node.js API for Qenlo. */
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import koffi from "koffi";

export interface RecordInput {
  readonly id: bigint;
  readonly userId: bigint;
  readonly timestamp: bigint;
  readonly vector: readonly number[];
}

export interface Filter {
  readonly userId?: bigint;
  readonly timestampLower?: bigint;
  readonly timestampUpper?: bigint;
}

export interface SearchResult {
  readonly id: bigint;
  readonly distance: number;
}

export interface ExecutionReport {
  readonly operationId: bigint;
  readonly requestedBackend: string;
  readonly actualBackend: string;
  readonly algorithm: string;
  readonly filterExecution: string;
  readonly indexGeneration: bigint;
  readonly rebuilt: boolean;
  readonly routingReason: string | null;
  readonly fallbackReason: string | null;
  readonly totalDurationNs: bigint;
  readonly lockWaitNs: bigint;
  readonly eligibleRows: bigint | null;
  readonly uploadBytes: bigint | null;
  readonly readbackBytes: bigint | null;
  readonly allocationBytes: bigint | null;
  readonly dispatchCount: number | null;
  readonly candidates: bigint | null;
  readonly batchSize: number;
}

export interface SearchResponse {
  readonly results: readonly SearchResult[];
  readonly report: ExecutionReport;
}

export interface CollectionStats {
  readonly dimension: number;
  readonly rows: number;
  readonly liveRows: number;
  readonly generation: bigint;
  readonly preparedGeneration: bigint | null;
  readonly durableGeneration: bigint | null;
  readonly recoveredInterruptedWrite: boolean;
  readonly closed: boolean;
}

export class QenloError extends Error {
  override readonly name = "QenloError";
}

type Pointer = unknown;
type NativeSearch = {
  results: Array<{ id: string; distance: number }>;
  report: Record<string, unknown>;
};

function libraryName(): string {
  switch (process.platform) {
    case "win32": return "qenlo_ffi.dll";
    case "darwin": return "libqenlo_ffi.dylib";
    default: return "libqenlo_ffi.so";
  }
}

function libraryPath(): string {
  if (process.env.QENLO_LIBRARY_PATH) return resolve(process.env.QENLO_LIBRARY_PATH);
  const here = dirname(fileURLToPath(import.meta.url));
  const candidates = [
    join(here, "..", "native", `${process.platform}-${process.arch}`, libraryName()),
    join(here, "..", "native", libraryName()),
    join(here, "..", "..", "..", "target", "release", libraryName()),
  ];
  const found = candidates.find(existsSync);
  if (!found) throw new QenloError(`Qenlo native library not found. Set QENLO_LIBRARY_PATH (${libraryName()}).`);
  return found;
}

const library = koffi.load(libraryPath());
const collectionNew = library.func("void *qenlo_collection_new(size_t)") as (dimension: number) => Pointer | null;
const collectionCreate = library.func("void *qenlo_collection_create(const char *, size_t)") as (path: string, dimension: number) => Pointer | null;
const collectionOpen = library.func("void *qenlo_collection_open(const char *, size_t)") as (path: string, dimension: number) => Pointer | null;
const nativeAdd = library.func("int32_t qenlo_add(void *, uint64_t, uint64_t, int64_t, const float *, size_t)") as (handle: Pointer, id: bigint, userId: bigint, timestamp: bigint, vector: Float32Array, length: number) => number;
const nativeAddBatch = library.func("int32_t qenlo_add_batch(void *, const uint64_t *, const uint64_t *, const int64_t *, const float *, size_t, size_t)") as (handle: Pointer, ids: BigUint64Array, users: BigUint64Array, timestamps: BigInt64Array, vectors: Float32Array, rows: number, dimension: number) => number;
const nativeDelete = library.func("int32_t qenlo_delete(void *, uint64_t)") as (handle: Pointer, id: bigint) => number;
const nativeDeleteBatch = library.func("int32_t qenlo_delete_batch(void *, const uint64_t *, size_t)") as (handle: Pointer, ids: BigUint64Array, rows: number) => number;
const nativeFlush = library.func("int32_t qenlo_flush(void *)") as (handle: Pointer) => number;
const nativeClose = library.func("int32_t qenlo_close(void *)") as (handle: Pointer) => number;
const nativeCollectionFree = library.func("void qenlo_collection_free(void *)") as (handle: Pointer) => void;
const nativeStringFree = library.func("void qenlo_string_free(void *)") as (value: Pointer) => void;
const QenloString = koffi.disposable("QenloString", "str", nativeStringFree);
const nativeSearch = library.func("qenlo_search", QenloString, ["void *", "const float *", "size_t", "bool", "uint64_t", "bool", "int64_t", "bool", "int64_t", "size_t"]) as (handle: Pointer, query: Float32Array, length: number, hasUser: boolean, user: bigint, hasLower: boolean, lower: bigint, hasUpper: boolean, upper: bigint, k: number) => string | null;
const nativeStats = library.func("qenlo_stats", QenloString, ["void *"]) as (handle: Pointer) => string | null;
const nativeLastError = library.func("qenlo_last_error", QenloString, []) as () => string | null;

function takeString(value: string | null): string {
  if (value === null) throw new QenloError(lastError());
  return value;
}

function lastError(): string {
  return nativeLastError() ?? "unknown Qenlo native error";
}

function optionalBigInt(value: unknown): bigint | null {
  return value === null ? null : BigInt(value as string);
}

function parseReport(value: Record<string, unknown>): ExecutionReport {
  return {
    operationId: BigInt(value.operation_id as string),
    requestedBackend: value.requested_backend as string,
    actualBackend: value.actual_backend as string,
    algorithm: value.algorithm as string,
    filterExecution: value.filter_execution as string,
    indexGeneration: BigInt(value.index_generation as string),
    rebuilt: value.rebuilt as boolean,
    routingReason: value.routing_reason as string | null,
    fallbackReason: value.fallback_reason as string | null,
    totalDurationNs: BigInt(value.total_duration_ns as string),
    lockWaitNs: BigInt(value.lock_wait_ns as string),
    eligibleRows: optionalBigInt(value.eligible_rows),
    uploadBytes: optionalBigInt(value.upload_bytes),
    readbackBytes: optionalBigInt(value.readback_bytes),
    allocationBytes: optionalBigInt(value.allocation_bytes),
    dispatchCount: value.dispatch_count as number | null,
    candidates: optionalBigInt(value.candidates),
    batchSize: value.batch_size as number,
  };
}

export class Collection implements Disposable {
  readonly dimension: number;
  #handle: Pointer | null;

  private constructor(handle: Pointer | null, dimension: number) {
    if (handle === null) throw new QenloError(lastError());
    this.#handle = handle;
    this.dimension = dimension;
  }

  static memory(dimension: number): Collection {
    Collection.validateDimension(dimension);
    return new Collection(collectionNew(dimension), dimension);
  }

  static create(path: string, dimension: number): Collection {
    Collection.validateDimension(dimension);
    return new Collection(collectionCreate(path, dimension), dimension);
  }

  static open(path: string, dimension: number): Collection {
    Collection.validateDimension(dimension);
    return new Collection(collectionOpen(path, dimension), dimension);
  }

  private static validateDimension(dimension: number): void {
    if (!Number.isSafeInteger(dimension) || dimension <= 0) throw new RangeError("dimension must be a positive safe integer");
  }

  #openHandle(): Pointer {
    if (this.#handle === null) throw new QenloError("collection is closed");
    return this.#handle;
  }

  #vector(vector: readonly number[]): Float32Array {
    if (vector.length !== this.dimension) throw new RangeError(`expected vector dimension ${this.dimension}, got ${vector.length}`);
    return Float32Array.from(vector);
  }

  #check(status: number): void {
    if (status !== 0) throw new QenloError(lastError());
  }

  add(record: RecordInput): void {
    const vector = this.#vector(record.vector);
    this.#check(nativeAdd(this.#openHandle(), record.id, record.userId, record.timestamp, vector, vector.length));
  }

  addBatch(records: readonly RecordInput[]): void {
    if (records.length === 0) return;
    const ids = new BigUint64Array(records.length);
    const users = new BigUint64Array(records.length);
    const timestamps = new BigInt64Array(records.length);
    const vectors = new Float32Array(records.length * this.dimension);
    records.forEach((record, row) => {
      ids[row] = record.id;
      users[row] = record.userId;
      timestamps[row] = record.timestamp;
      vectors.set(this.#vector(record.vector), row * this.dimension);
    });
    this.#check(nativeAddBatch(this.#openHandle(), ids, users, timestamps, vectors, records.length, this.dimension));
  }

  delete(id: bigint): void {
    this.#check(nativeDelete(this.#openHandle(), id));
  }

  deleteBatch(ids: readonly bigint[]): void {
    if (ids.length === 0) return;
    const native = BigUint64Array.from(ids);
    this.#check(nativeDeleteBatch(this.#openHandle(), native, ids.length));
  }

  search(query: readonly number[], filter: Filter = {}, k = 10): SearchResponse {
    if (!Number.isSafeInteger(k) || k < 1 || k > 64) throw new RangeError("k must be in 1..=64");
    const vector = this.#vector(query);
    const value = JSON.parse(takeString(nativeSearch(
      this.#openHandle(), vector, vector.length,
      filter.userId !== undefined, filter.userId ?? 0n,
      filter.timestampLower !== undefined, filter.timestampLower ?? 0n,
      filter.timestampUpper !== undefined, filter.timestampUpper ?? 0n, k,
    ))) as NativeSearch;
    return {
      results: value.results.map((hit) => ({ id: BigInt(hit.id), distance: hit.distance })),
      report: parseReport(value.report),
    };
  }

  stats(): CollectionStats {
    const value = JSON.parse(takeString(nativeStats(this.#openHandle()))) as Record<string, unknown>;
    return {
      dimension: value.dimension as number,
      rows: value.rows as number,
      liveRows: value.live_rows as number,
      generation: BigInt(value.generation as string),
      preparedGeneration: optionalBigInt(value.prepared_generation),
      durableGeneration: optionalBigInt(value.durable_generation),
      recoveredInterruptedWrite: value.recovered_interrupted_write as boolean,
      closed: value.closed as boolean,
    };
  }

  flush(): void { this.#check(nativeFlush(this.#openHandle())); }

  close(): void {
    if (this.#handle !== null) {
      const handle = this.#handle;
      this.#handle = null;
      const status = nativeClose(handle);
      nativeCollectionFree(handle);
      this.#check(status);
    }
  }

  [Symbol.dispose](): void { this.close(); }
}
