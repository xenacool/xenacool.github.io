// Alpha-neighbor outline for the existing atlas texture. No second texture or
// SDF is required; transparent neighbors become a stable silhouette edge.
export function configureSpriteOutline(material, texture, atlasSize = [4080, 5372]) {
    material.onBeforeCompile = (shader) => {
        shader.uniforms.outlineColor = { value: [0.06, 0.08, 0.12] };
        shader.uniforms.outlineTexel = { value: [1 / atlasSize[0], 1 / atlasSize[1]] };
        shader.uniforms.outlineWidth = { value: 1.0 };
        shader.fragmentShader = shader.fragmentShader.replace(
            '#include <map_fragment>',
            `#include <map_fragment>
            float outlineAlpha = 0.0;
            vec2 outlineStep = outlineTexel * outlineWidth;
            outlineAlpha = max(outlineAlpha, texture2D(map, vMapUv + vec2(outlineStep.x, 0.0)).a);
            outlineAlpha = max(outlineAlpha, texture2D(map, vMapUv - vec2(outlineStep.x, 0.0)).a);
            outlineAlpha = max(outlineAlpha, texture2D(map, vMapUv + vec2(0.0, outlineStep.y)).a);
            outlineAlpha = max(outlineAlpha, texture2D(map, vMapUv - vec2(0.0, outlineStep.y)).a);
            if (diffuseColor.a < 0.01 && outlineAlpha > 0.01) diffuseColor = vec4(outlineColor, 1.0);`,
        );
        shader.fragmentShader = `uniform vec3 outlineColor; uniform vec2 outlineTexel; uniform float outlineWidth;\n${shader.fragmentShader}`;
        material.userData.outlineShader = shader;
    };
    material.needsUpdate = true;
    void texture;
}
