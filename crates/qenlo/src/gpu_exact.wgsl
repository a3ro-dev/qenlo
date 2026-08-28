struct Params {
    rows: u32,
    dimension: u32,
    mode: u32,
    eligible_count: u32,
    user_lo: u32,
    user_hi: u32,
    lower_lo: u32,
    lower_hi: u32,
    upper_lo: u32,
    upper_hi: u32,
    predicate_flags: u32,
    k: u32,
}

struct Candidate {
    distance: f32,
    row: u32,
    id_lo: u32,
    id_hi: u32,
}

@group(0) @binding(0) var<storage, read> vectors: array<f32>;
@group(0) @binding(1) var<storage, read> users: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read> timestamps: array<vec2<u32>>;
@group(0) @binding(3) var<storage, read> ids: array<vec2<u32>>;
@group(0) @binding(4) var<storage, read> query: array<f32>;
@group(0) @binding(5) var<storage, read> eligibility: array<u32>;
@group(0) @binding(6) var<storage, read_write> scores: array<f32>;
@group(0) @binding(7) var<uniform> params: Params;

fn u64_equal(a: vec2<u32>, lo: u32, hi: u32) -> bool {
    return a.x == lo && a.y == hi;
}

// Flipping the sign bit maps two's-complement i64 ordering to unsigned ordering.
fn i64_less(a: vec2<u32>, b_lo: u32, b_hi: u32) -> bool {
    let ah = a.y ^ 0x80000000u;
    let bh = b_hi ^ 0x80000000u;
    return ah < bh || (ah == bh && a.x < b_lo);
}

fn predicate_matches(row: u32) -> bool {
    let ts = timestamps[row];
    let user_matches = (params.predicate_flags & 1u) == 0u
        || u64_equal(users[row], params.user_lo, params.user_hi);
    let at_or_after_lower = (params.predicate_flags & 2u) == 0u
        || !i64_less(ts, params.lower_lo, params.lower_hi);
    let before_upper = (params.predicate_flags & 4u) == 0u
        || i64_less(ts, params.upper_lo, params.upper_hi);
    return eligibility[row] != 0u && user_matches
        && at_or_after_lower && before_upper;
}

// Eight vectors per workgroup, with 32 adjacent lanes loading adjacent dimensions.
// This is an ordinary shared-memory reduction; it needs no subgroup extension.
var<workgroup> partial_dot: array<f32, 256>;

@compute @workgroup_size(256)
fn score(@builtin(workgroup_id) group: vec3<u32>,
         @builtin(local_invocation_index) tid: u32) {
    let item = group.x * 8u + tid / 32u;
    let lane = tid % 32u;
    var row = item;
    var eligible = item < params.rows;
    if (params.mode == 1u) {
        eligible = item < params.eligible_count;
        if (eligible) { row = eligibility[item]; }
    }
    if (eligible) {
        if (params.mode == 0u) {
            eligible = eligibility[row] != 0u;
        } else if (params.mode == 2u) {
            eligible = predicate_matches(row);
        }
    }

    var dot = 0.0;
    if (eligible) {
        let offset = row * params.dimension;
        for (var d = lane; d < params.dimension; d += 32u) {
            dot += vectors[offset + d] * query[d];
        }
    }
    partial_dot[tid] = dot;
    workgroupBarrier();
    for (var stride = 16u; stride > 0u; stride /= 2u) {
        if (lane < stride) { partial_dot[tid] += partial_dot[tid + stride]; }
        workgroupBarrier();
    }
    let count = select(params.rows, params.eligible_count, params.mode == 1u);
    if (lane == 0u && item < count) {
        scores[item] = select(3.402823466e+38, 1.0 - partial_dot[tid], eligible);
    }
}

@group(1) @binding(0) var<storage, read_write> selected: array<Candidate>;

fn better(a: Candidate, b: Candidate) -> bool {
    return a.distance < b.distance || (a.distance == b.distance &&
        (a.id_hi < b.id_hi || (a.id_hi == b.id_hi && a.id_lo < b.id_lo)));
}

var<workgroup> best: array<Candidate, 256>;

// Each lane scans 1/256 of the rows, then a tree reduction elects the exact minimum.
// Consuming that score and repeating k times keeps readback at k candidates per chunk.
@compute @workgroup_size(256)
fn select_topk(@builtin(local_invocation_index) tid: u32) {
    let count = select(params.rows, params.eligible_count, params.mode == 1u);
    for (var out = 0u; out < params.k; out += 1u) {
        var local_best = Candidate(3.402823466e+38, 0xffffffffu, 0xffffffffu, 0xffffffffu);
        for (var item = tid; item < count; item += 256u) {
            var row = item;
            if (params.mode == 1u) { row = eligibility[item]; }
            let id = ids[row];
            let candidate = Candidate(scores[item], item, id.x, id.y);
            if (better(candidate, local_best)) { local_best = candidate; }
        }
        best[tid] = local_best;
        workgroupBarrier();
        for (var stride = 128u; stride > 0u; stride /= 2u) {
            if (tid < stride && better(best[tid + stride], best[tid])) {
                best[tid] = best[tid + stride];
            }
            workgroupBarrier();
        }
        if (tid == 0u) {
            var winner = best[0];
            if (winner.distance == 3.402823466e+38) {
                winner.row = 0xffffffffu;
            } else {
                scores[winner.row] = 3.402823466e+38;
            }
            selected[out] = winner;
        }
        storageBarrier();
        workgroupBarrier();
    }
}
