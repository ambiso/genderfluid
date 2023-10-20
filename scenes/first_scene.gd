extends Node3D


# Called when the node enters the scene tree for the first time.
func _ready():
	# Create a local rendering device.
	var rd := RenderingServer.create_local_rendering_device()
	# Load GLSL shader
	var shader_file := load("res://compute_example.glsl")
	var shader_spirv: RDShaderSPIRV = shader_file.get_spirv()
	var shader := rd.shader_create_from_spirv(shader_spirv)
	# Prepare our data. We use floats in the shader, so we need 32 bit.
	var input := PackedFloat32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
	var input_bytes := input.to_byte_array()

	# Create a storage buffer that can hold our float values.
	# Each float has 4 bytes (32 bit) so 10 x 4 = 40 bytes
	var buffer := rd.storage_buffer_create(input_bytes.size(), input_bytes)


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
