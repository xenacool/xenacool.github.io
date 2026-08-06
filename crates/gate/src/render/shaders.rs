pub const VERTEX_SHADER: &str = "precision mediump float;
attribute vec3 aPosition;
attribute vec3 aNormal;
attribute vec2 aUV;

uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProjection;

varying vec3 vNormal;
varying vec3 vFragPos;
varying vec2 vUV;
varying float vHeight;

void main() {
    vec4 worldPos = uModel * vec4(aPosition, 1.0);
    vFragPos = worldPos.xyz;
    vNormal = mat3(uModel) * aNormal;
    vUV = aUV;
    vHeight = worldPos.y;
    gl_Position = uProjection * uView * worldPos;
}";

pub const FRAGMENT_SHADER: &str = "precision mediump float;

struct Light {
    vec3 direction;
    vec3 color;
    float intensity;
};

varying vec3 vNormal;
varying vec3 vFragPos;
varying vec2 vUV;
varying float vHeight;

uniform Light uLights[4];
uniform vec3 uAmbientColor;
uniform float uAmbientIntensity;

uniform mat4 uModel;
uniform vec3 uObjectColor;
uniform bool uUseTexture;
uniform sampler2D uTexture;
uniform bool uUseNormalMap;
uniform sampler2D uNormalMap;

uniform float uRoughness;
uniform float uMetalness;
uniform float uEmissive;

void main() {
    vec3 norm = normalize(vNormal);
    if (uUseNormalMap) {
        vec3 normalSample = texture2D(uNormalMap, vUV).rgb * 2.0 - 1.0;
        // Standard normal map: Z is UP (out of the slice).
        // Quad rotated -90 around X: Quad local Z is World UP (+Y).
        vec3 quadNormal = vec3(normalSample.x, normalSample.y, normalSample.z);
        norm = normalize(mat3(uModel) * quadNormal);
    }
    vec3 viewDir = normalize(vec3(0.0, 1.0, 1.0)); // Fixed camera-ish
    
    vec3 linearAmbient = pow(uAmbientColor, vec3(2.2));
    vec3 lighting = linearAmbient * uAmbientIntensity;

    for (int i = 0; i < 4; i++) {
        vec3 L = normalize(uLights[i].direction);
        vec3 linearLightColor = pow(uLights[i].color, vec3(2.2));
        // Diffuse
        float diff = max(dot(norm, L), 0.0);
        
        // Specular
        vec3 reflectDir = reflect(-L, norm);
        float spec = pow(max(dot(viewDir, reflectDir), 0.0), (1.0 - uRoughness) * 128.0);
        vec3 specular = uMetalness * spec * linearLightColor;
        
        lighting += ((diff * linearLightColor) + specular) * uLights[i].intensity;
    }

    vec3 color = pow(uObjectColor, vec3(2.2));
    float alpha = 1.0;
    if (uUseTexture) {
        vec4 texColor = texture2D(uTexture, vUV);
        if (texColor.a < 0.01) discard;
        vec3 linearTexColor = pow(texColor.rgb, vec3(2.2));
        color = mix(color, linearTexColor, texColor.a);
        alpha = texColor.a;
    }

    vec3 result = lighting * color + (uEmissive * color);
    
    // Height-based tint/fade
    result *= (0.9 + 0.2 * vHeight);

    gl_FragColor = vec4(pow(result, vec3(1.0/2.2)), alpha);
}";
