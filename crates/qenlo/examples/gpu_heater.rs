//! Research tool: a duty-cycled GPU load that raises the GPU's DVFS clock state
//! without touching the process being measured. Not part of the library.
//!
//!     cargo run --release -p qenlo --features gpu-wgpu --example gpu_heater -- ITERS SLEEP_MS SECONDS
//!
//! Each cycle dispatches one busy kernel (ITERS dependent FMAs per thread over
//! 64 workgroups), waits for it, then sleeps SLEEP_MS. Prints the measured busy time.
use std::{future::Future, pin::pin, task::{Context, Poll, Waker}, time::{Duration, Instant}};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(v) = future.as_mut().poll(&mut cx) { return v; }
        std::thread::yield_now();
    }
}

const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> out: array<f32>;
struct P { iters: u32 }
@group(0) @binding(1) var<uniform> p: P;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var x = f32(id.x) * 1e-6;
    for (var i = 0u; i < p.iters; i++) { x = fma(x, 0.999999, 1e-7); }
    out[id.x] = x;
}
"#;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|a| a.parse().expect("integer args")).collect();
    let (iters, sleep_ms, seconds) = (args[0] as u32, args[1], args[2]);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    })).expect("adapter");
    eprintln!("heater adapter: {:?}", adapter.get_info().name);
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).expect("device");
    let threads = 64 * 256u64;
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: threads * 4, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
    let params = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    queue.write_buffer(&params, 0, &[iters.to_le_bytes(), [0; 4], [0; 4], [0; 4]].concat());
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None, layout: None, module: &module, entry_point: Some("main"),
        compilation_options: Default::default(), cache: None });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: out.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: params.as_entire_binding() },
        ] });
    let (start, mut busy, mut cycles) = (Instant::now(), Duration::ZERO, 0u64);
    while start.elapsed().as_secs() < seconds {
        let t = Instant::now();
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(64, 1, 1);
        }
        queue.submit([enc.finish()]);
        device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).expect("poll");
        busy += t.elapsed();
        cycles += 1;
        std::thread::sleep(Duration::from_millis(sleep_ms));
    }
    eprintln!("heater cycles={cycles} mean_busy_ms={:.3} duty={:.3}",
        busy.as_secs_f64() * 1e3 / cycles as f64, busy.as_secs_f64() / start.elapsed().as_secs_f64());
}
