// Alpha-neighbor outline for the existing atlas texture. No second texture or
// SDF is required; transparent neighbors become a stable silhouette edge.
export function configureSpriteOutline(material, texture, atlasSize = [4080, 5372], region = null) {
    material.onBeforeCompile = (shader) => {
        shader.uniforms.outlineColor = { value: [0.02, 0.015, 0.01] };
        shader.uniforms.outlineTexel = { value: [1 / atlasSize[0], 1 / atlasSize[1]] };
        shader.uniforms.outlineWidth = { value: 2.0 };
        shader.uniforms.outlineUvMin = { value: [region?.u0 ?? 0, region?.v0 ?? 0] };
        shader.uniforms.outlineUvMax = { value: [region?.u1 ?? 1, region?.v1 ?? 1] };
        shader.fragmentShader = shader.fragmentShader.replace(
            '#include <map_fragment>',
            `#include <map_fragment>
            float outlineAlpha = 0.0;
            vec2 outlineStep = outlineTexel * outlineWidth;
            vec2 outlineUv = clamp(vMapUv, outlineUvMin, outlineUvMax);
            float outlineNeighbors = 0.0;
            outlineNeighbors += texture2D(map, clamp(outlineUv + vec2(outlineStep.x, 0.0), outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv - vec2(outlineStep.x, 0.0), outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv + vec2(0.0, outlineStep.y), outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv - vec2(0.0, outlineStep.y), outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv + outlineStep, outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv - outlineStep, outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv + vec2(outlineStep.x, -outlineStep.y), outlineUvMin, outlineUvMax)).a;
            outlineNeighbors += texture2D(map, clamp(outlineUv + vec2(-outlineStep.x, outlineStep.y), outlineUvMin, outlineUvMax)).a;
            if (diffuseColor.a < 0.01 && outlineNeighbors > 0.25) diffuseColor = vec4(outlineColor, 1.0);`,
        );
        shader.fragmentShader = `uniform vec3 outlineColor; uniform vec2 outlineTexel; uniform float outlineWidth; uniform vec2 outlineUvMin; uniform vec2 outlineUvMax;\n${shader.fragmentShader}`;
        material.userData.outlineShader = shader;
    };
    material.needsUpdate = true;
    void texture;
}
