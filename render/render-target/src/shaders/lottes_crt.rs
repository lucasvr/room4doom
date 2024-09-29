use std::f32::consts::FRAC_PI_4;

use gameplay::glam::{Mat4, Vec3};
use golem::Dimension::*;
use golem::*;

use super::{ShaderDraw, GL_QUAD, GL_QUAD_INDICES};

pub struct LottesCRT {
    _quad: [f32; 16],
    indices: [u32; 6],
    crt_shader: ShaderProgram,
    projection: Mat4,
    look_at: Mat4,
    vb: VertexBuffer,
    eb: ElementBuffer,
}

impl LottesCRT {
    pub fn new(ctx: &Context) -> Self {
        let shader = ShaderDescription {
            uniforms: &[
                // Standard view stuff
                Uniform::new("projMat", UniformType::Matrix(D4)),
                Uniform::new("viewMat", UniformType::Matrix(D4)),
                Uniform::new("modelMat", UniformType::Matrix(D4)),
                //
                Uniform::new(
                    "color_texture_sz",
                    UniformType::Vector(NumberType::Float, D2),
                ),
                Uniform::new(
                    "color_texture_pow2_sz",
                    UniformType::Vector(NumberType::Float, D2),
                ),
                //
                Uniform::new("hardScan", UniformType::Scalar(NumberType::Float)),
                Uniform::new("hardPix", UniformType::Scalar(NumberType::Float)),
                Uniform::new("maskDark", UniformType::Scalar(NumberType::Float)),
                Uniform::new("maskLight", UniformType::Scalar(NumberType::Float)),
                Uniform::new("saturation", UniformType::Scalar(NumberType::Float)),
                Uniform::new("tint", UniformType::Scalar(NumberType::Float)),
                Uniform::new("blackClip", UniformType::Scalar(NumberType::Float)),
                Uniform::new("brightMult", UniformType::Scalar(NumberType::Float)),
                Uniform::new("distortion", UniformType::Scalar(NumberType::Float)),
                Uniform::new("cornersize", UniformType::Scalar(NumberType::Float)),
                Uniform::new("cornersmooth", UniformType::Scalar(NumberType::Float)),
                Uniform::new("toSRGB", UniformType::Scalar(NumberType::Float)),
                // The SDL bytes
                Uniform::new("image", UniformType::Sampler2D),
            ],
            vertex_input: &[
                Attribute::new("position", AttributeType::Vector(D2)),
                Attribute::new("vert_uv", AttributeType::Vector(D2)),
            ],
            vertex_shader: VERT,
            fragment_input: &[Attribute::new("texCoord", AttributeType::Vector(D2))],
            fragment_shader: FRAG,
        };
        let shader = ShaderProgram::new(ctx, shader).unwrap();

        let projection = Mat4::perspective_rh_gl(FRAC_PI_4, 1.0, 0.1, 50.0);
        let look_at = Mat4::look_at_rh(
            Vec3::new(0.0, 0.0, 2.42),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        let mut vb = VertexBuffer::new(ctx).unwrap();
        let mut eb = ElementBuffer::new(ctx).unwrap();
        vb.set_data(&GL_QUAD);
        eb.set_data(&GL_QUAD_INDICES);

        Self {
            _quad: GL_QUAD,
            indices: GL_QUAD_INDICES,
            crt_shader: shader,
            projection,
            look_at,
            vb,
            eb,
        }
    }
}

impl ShaderDraw for LottesCRT {
    fn draw(&mut self, texture: &Texture) -> Result<(), GolemError> {
        self.crt_shader.bind();
        self.crt_shader.prepare_draw(&self.vb, &self.eb)?;

        self.crt_shader.set_uniform("image", UniformValue::Int(1))?;

        self.crt_shader.set_uniform(
            "projMat",
            UniformValue::Matrix4(self.projection.to_cols_array()),
        )?;
        self.crt_shader.set_uniform(
            "viewMat",
            UniformValue::Matrix4(self.look_at.to_cols_array()),
        )?;
        self.crt_shader.set_uniform(
            "modelMat",
            UniformValue::Matrix4(Mat4::IDENTITY.to_cols_array()),
        )?;

        // CRT settings
        self.crt_shader.set_uniform(
            "color_texture_sz",
            UniformValue::Vector2([texture.width() as f32, texture.height() as f32]),
        )?;

        // size of color texture rounded up to power of 2
        self.crt_shader.set_uniform(
            "color_texture_pow2_sz",
            UniformValue::Vector2([texture.width() as f32, texture.height() as f32]),
        )?;

        // Hardness of scanline.
        //  -8.0 = soft
        // -16.0 = medium
        self.crt_shader
            .set_uniform("hardScan", UniformValue::Float(-8.0))?;

        // Hardness of pixels in scanline.
        // -2.0 = soft
        // -4.0 = hard
        self.crt_shader
            .set_uniform("hardPix", UniformValue::Float(-3.0))?;

        // Amount of shadow mask
        self.crt_shader
            .set_uniform("maskDark", UniformValue::Float(1.1))?;
        self.crt_shader
            .set_uniform("maskLight", UniformValue::Float(1.5))?;

        // GAMMA, needs to be increased if SRGB not used
        self.crt_shader
            .set_uniform("brightMult", UniformValue::Float(0.2))?;
        self.crt_shader
            .set_uniform("toSRGB", UniformValue::Float(1.0))?;

        // SHAPE
        self.crt_shader
            .set_uniform("distortion", UniformValue::Float(0.07))?; // 0.05 to 0.3

        self.crt_shader
            .set_uniform("cornersize", UniformValue::Float(0.022))?; // 0.01 to 0.05

        // Edge hardness
        self.crt_shader
            .set_uniform("cornersmooth", UniformValue::Float(70.0))?; // 70.0 to 170.0

        let bind_point = std::num::NonZeroU32::new(1).unwrap();
        texture.set_active(bind_point);

        unsafe {
            self.crt_shader
                .draw_prepared(0..self.indices.len(), GeometryMode::Triangles);
        }
        Ok(())
    }
}

const FRAG: &str = r#"
#pragma optimize (on)
#pragma debug (off)

//An extra per channel gamma adjustment applied at the end.
const vec3 gammaBoost = vec3(1.0/1.2, 1.0/1.2, 1.0/1.2);

// sRGB to Linear.
// Assuing using sRGB typed textures this should not be needed.
float ToLinear1(float c)
{
    return(c <= 0.04045) ? c / 12.92 : pow((c+0.055) / 1.055,2.4);
}
vec3 ToLinear(vec3 c)
{
    return vec3( ToLinear1(c.r), ToLinear1(c.g), ToLinear1(c.b) );
}

// Linear to sRGB.
// Assuming using sRGB typed textures this should not be needed.
float ToSrgb1(float c)
{
    return( c < 0.0031308 ? c * 12.92 : 1.055 * pow(c,0.41666) - 0.055);
}
vec3 ToSrgb(vec3 c)
{
    return vec3(ToSrgb1(c.r), ToSrgb1(c.g), ToSrgb1(c.b));
}

// Nearest emulated sample given floating point position and texel offset.
// Also zero's off screen.
vec3 Fetch(vec2 pos, vec2 off)
{
    pos = (floor(pos * color_texture_pow2_sz + off) + 0.5) / color_texture_pow2_sz;
    if(max(abs(pos.x-0.5),abs(pos.y-0.5))>0.5)return vec3(0.0,0.0,0.0);
    return ToLinear(texture2D(image, pos.xy).rgb);
}

// Distance in emulated pixels to nearest texel.
vec2 Dist(vec2 pos)
{
    pos = pos * color_texture_pow2_sz;
    return -((pos - floor(pos)) - vec2(0.5));
}

// 1D Gaussian.
float Gaus(float pos,float scale)
{
    return exp2(scale * pos * pos);
}

// 3-tap Gaussian filter along horz line.
vec3 Horz3(vec2 pos,float off)
{
    vec3 b = Fetch(pos, vec2(-1.0, off));
    vec3 c = Fetch(pos, vec2( 0.0, off));
    vec3 d = Fetch(pos, vec2( 1.0, off));
    float dst = Dist(pos).x;
    // Convert distance to weight.
    float scale = hardPix;
    float wb = Gaus(dst - 1.0, scale);
    float wc = Gaus(dst + 0.0, scale);
    float wd = Gaus(dst + 1.0, scale);
    // Return filtered sample.
    return (b * wb + c * wc + d * wd) / (wb + wc + wd);
}

// 5-tap Gaussian filter along horz line.
vec3 Horz5(vec2 pos,float off)
{
    vec3 a = Fetch(pos, vec2(-2.0, off));
    vec3 b = Fetch(pos, vec2(-1.0, off));
    vec3 c = Fetch(pos, vec2( 0.0, off));
    vec3 d = Fetch(pos, vec2( 1.0, off));
    vec3 e = Fetch(pos, vec2( 2.0, off));
    float dst = Dist(pos).x;
    // Convert distance to weight.
    float scale = hardPix;
    float wa = Gaus(dst - 2.0, scale);
    float wb = Gaus(dst - 1.0, scale);
    float wc = Gaus(dst + 0.0, scale);
    float wd = Gaus(dst + 1.0, scale);
    float we = Gaus(dst + 2.0, scale);
    // Return filtered sample.
    return (a * wa + b * wb + c * wc + d * wd + e * we) / (wa + wb + wc + wd + we);
}

// Return scanline weight.
float Scan(vec2 pos,float off)
{
    float dst = Dist(pos).y;
    vec3 col = Fetch(pos,vec2(0.0));
    return Gaus( dst + off, hardScan / (dot(col, col) * 0.1667 + 1.0) );
    // Modified to make scanline respond to pixel brightness
}

// Allow nearest three lines to effect pixel.
vec3 Tri(vec2 pos)
{
    vec3 a = Horz3(pos, -1.0);
    vec3 b = Horz5(pos, 0.0);
    vec3 c = Horz3(pos, 1.0);
    float wa = Scan(pos, -1.0);
    float wb = Scan(pos, 0.0);
    float wc = Scan(pos, 1.0);
    return a * wa + b * wb + c * wc;
}

// Shadow mask.
vec3 Mask(vec2 pos)
{
    // VGA style shadow mask.
    pos.xy = floor(pos.xy * vec2(1.0, 0.5));
    pos.x += pos.y * 3.0;
    vec3 mask = vec3(maskDark, maskDark, maskDark);
    pos.x = fract(pos.x / 6.0);
    if (pos.x<0.333)
        mask.r = maskLight;
    else if (pos.x < 0.666)
        mask.g = maskLight;
    else
        mask.b = maskLight;
    return mask;
}

///////////////////////////////////////////////////////////////
/// CRT GEOM FUNCTIONS ///
vec2 radialDistortion(vec2 coord) {
    coord *= color_texture_pow2_sz / color_texture_sz;
    vec2 cc = coord - vec2(0.5);
    float dist = dot(cc, cc) * distortion;
    return (coord + cc * (1.0 + dist) * dist) * color_texture_sz / color_texture_pow2_sz;
}

float corner(vec2 coord)
{
    coord *= color_texture_pow2_sz / color_texture_sz;
    coord = (coord - vec2(0.5)) + vec2(0.5);
    coord = min(coord, vec2(1.0)-coord);
    vec2 cdist = vec2(cornersize);
    coord = (cdist - min(coord,cdist));
    float dist = sqrt(dot(coord,coord));
    return clamp((cdist.x-dist)*cornersmooth,0.0, 1.0);
}
///////////////////////////////////////////////////////////////

void main(void)
{
    gl_FragColor.a = 1.0;
    vec2 pos = radialDistortion(texCoord);
    gl_FragColor.rgb = Tri(pos) * Mask(gl_FragCoord.xy) * vec3(corner(pos));
    gl_FragColor.rgb += brightMult*pow(gl_FragColor.rgb,gammaBoost);

    if (toSRGB == 1.0) {
        gl_FragColor.rgb = ToSrgb(gl_FragColor.rgb);
    }
}"#;

const VERT: &str = r#"
void main() {
    gl_Position = projMat * viewMat * modelMat * vec4(position, 0.0, 1.0);
    texCoord = vert_uv;
}"#;
