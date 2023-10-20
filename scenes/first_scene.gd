extends Node3D
# Member variables for resources that will be reused each frame
var rd
var heightFieldBuffer1
var heightFieldBuffer2
var velocityBuffer
var waterComputeShader
var vertex_shader
var output_image = Image.new()
var output_texture = ImageTexture.new()

var curBuf = 1

func flipBuffers():
	if curBuf == 1:
		curBuf = 2
	else:
		curBuf = 1
		
func getHeightFieldOutputBuf():
	if curBuf == 1:
		return heightFieldBuffer2
	else:
		return heightFieldBuffer1
		

func getHeightFieldInputBuf():
	if curBuf == 1:
		return heightFieldBuffer1
	else:
		return heightFieldBuffer2

func _ready():
	# Initialization code
	rd = RenderingServer.create_local_rendering_device()

	var shader_file = load("res://shaders/water_compute.glsl")
	var shader_spirv: RDShaderSPIRV = shader_file.get_spirv()
	print(shader_spirv.compile_error_compute)
	waterComputeShader = rd.shader_create_from_spirv(shader_spirv)

	var heightField = PackedFloat32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
	var heightFieldBytes = heightField.to_byte_array()
	heightFieldBuffer1 = rd.storage_buffer_create(heightFieldBytes.size(), heightFieldBytes)
	heightFieldBuffer2 = rd.storage_buffer_create(heightFieldBytes.size(), heightFieldBytes)
	
	var inputVel = PackedFloat32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
	var input_bytes_vel = inputVel.to_byte_array()
	velocityBuffer = rd.storage_buffer_create(input_bytes_vel.size(), input_bytes_vel)
	
	# Initialize the output image with dimensions and format (let's say 256x256)
	output_image.create(256, 256, false, Image.FORMAT_RGBA8)
	output_texture.create_from_image(output_image)
	
	var shader_code = ""
	var file = FileAccess.open("res://shaders/vertex.glsl", FileAccess.READ)

	if file:
		shader_code = file.get_as_text()
		file.close()
	else:
		print("Failed to read shader file")
	vertex_shader = Shader.new()
	vertex_shader.set_code(shader_code)
	
	var material = ShaderMaterial.new()
	material.shader = vertex_shader
	self.material_override = material


func _process(delta):
	var heightFieldInputUniform = RDUniform.new()
	heightFieldInputUniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	heightFieldInputUniform.binding = 0
	heightFieldInputUniform.add_id(getHeightFieldInputBuf())
	
	var heightFieldOutputUniform = RDUniform.new()
	heightFieldOutputUniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	heightFieldOutputUniform.binding = 1
	heightFieldOutputUniform.add_id(getHeightFieldOutputBuf())
	
	var velocityFieldInputUniform = RDUniform.new()
	velocityFieldInputUniform.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	velocityFieldInputUniform.binding = 2
	velocityFieldInputUniform.add_id(velocityBuffer)

	var uniform_set = rd.uniform_set_create([heightFieldInputUniform, velocityFieldInputUniform, heightFieldOutputUniform], waterComputeShader, 0)
	var pipeline = rd.compute_pipeline_create(waterComputeShader)
	
	var compute_list = rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(compute_list, pipeline)
	rd.compute_list_bind_uniform_set(compute_list, uniform_set, 0)
	rd.compute_list_dispatch(compute_list, 5, 1, 1)
	rd.compute_list_end()

	var output_bytes = rd.buffer_get_data(getHeightFieldOutputBuf())
	var output = output_bytes.to_float32_array()

	
	#print("Output: ", output)
	flipBuffers()
