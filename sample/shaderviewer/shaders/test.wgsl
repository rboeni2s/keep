struct VertexOutput
{
    @builtin(position) position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
};


@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput
{
    var out: VertexOutput;

    let x = f32(1 - i32(vertex_index)) * 0.5;
    let y = f32(i32(vertex_index & 1u) * 2 - 1) * 0.5;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.position.xyz;

    return out;
}


@fragment
fn fragment_main(vertex_output: VertexOutput) -> @location(0) vec4<f32>
{
    return vec4<f32>(0.9, 0.2, 0.2, 1.0);
}
