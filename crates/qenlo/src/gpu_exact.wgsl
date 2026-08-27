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

@compute @workgroup_size(256)
fn init_scores(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x < params.rows) {
        scores[gid.x] = 3.402823466e+38;
    }
}

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

@compute @workgroup_size(256)
fn score(@builtin(global_invocation_id) gid: vec3<u32>) {
    let item = gid.x;
    var row = item;
    if (params.mode == 1u) {
        if (item >= params.eligible_count) { return; }
        row = eligibility[item];
    } else if (item >= params.rows) {
        return;
    }

    var eligible = true;
    if (params.mode == 0u) {
        eligible = eligibility[row] != 0u;
    } else if (params.mode == 2u) {
        eligible = predicate_matches(row);
    }
    if (!eligible || row >= params.rows) {
        return;
    }

    var dot = 0.0;
    let offset = row * params.dimension;
    for (var d = 0u; d < params.dimension; d += 1u) {
        dot += vectors[offset + d] * query[d];
    }
    scores[row] = 1.0 - dot;
}

@group(1) @binding(0) var<storage, read_write> selected: array<Candidate>;

// This intentionally simple reference selector makes readback bounded. It is O(rows*k)
// and should be replaced only after the experiment establishes that selection dominates.
@compute @workgroup_size(1)
fn select() {
    for (var out = 0u; out < params.k; out += 1u) {
        var best_distance = 3.402823466e+38;
        var best_row = 0xffffffffu;
        var best_id = vec2<u32>(0xffffffffu, 0xffffffffu);
        for (var row = 0u; row < params.rows; row += 1u) {
            let distance = scores[row];
            let id = ids[row];
            var used = false;
            for (var prior = 0u; prior < out; prior += 1u) {
                used = used || selected[prior].row == row;
            }
            let id_less = id.y < best_id.y || (id.y == best_id.y && id.x < best_id.x);
            if (!used && (distance < best_distance || (distance == best_distance && id_less))) {
                best_distance = distance;
                best_row = row;
                best_id = id;
            }
        }
        if (best_distance == 3.402823466e+38) {
            best_row = 0xffffffffu;
            best_id = vec2<u32>(0xffffffffu, 0xffffffffu);
        }
        selected[out].distance = best_distance;
        selected[out].row = best_row;
        selected[out].id_lo = best_id.x;
        selected[out].id_hi = best_id.y;
    }
}
