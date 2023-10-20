extends Node3D
# Member variables for resources that will be reused each frame
var rd
var buffer
var buffer_vel
var pipeline
var uniform_set

func _ready():
	# Initialization code
	rd = RenderingServer.create_local_rendering_device()

	var shader_file = load("res://shaders/water_compute.glsl")
	var shader_spirv: RDShaderSPIRV = shader_file.get_spirv()
	var shader = rd.shader_create_from_spirv(shader_spirv)

	var input = PackedFloat32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
	var input_bytes = input.to_byte_array()
	buffer = rd.storage_buffer_create(input_bytes.size(), input_bytes)

	var uniform = RDUniform.new()
	uniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	uniform.binding = 0
	uniform.add_id(buffer)
	
	var inputVel = PackedFloat32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
	var input_bytes_vel = inputVel.to_byte_array()
	buffer_vel = rd.storage_buffer_create(input_bytes_vel.size(), input_bytes_vel)

	var uniform_vel = RDUniform.new()
	uniform_vel.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	uniform_vel.binding = 1
	uniform_vel.add_id(buffer_vel)

	uniform_set = rd.uniform_set_create([uniform, uniform_vel], shader, 0)
	pipeline = rd.compute_pipeline_create(shader)

func _process(delta):
	# Per-frame code
	var compute_list = rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(compute_list, pipeline)
	rd.compute_list_bind_uniform_set(compute_list, uniform_set, 0)
	rd.compute_list_dispatch(compute_list, 5, 1, 1)
	rd.compute_list_end()

	var output_bytes = rd.buffer_get_data(buffer)
	var output = output_bytes.to_float32_array()
	print("Output: ", output)
