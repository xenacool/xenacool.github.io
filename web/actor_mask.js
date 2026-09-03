// CPU reference for the actor silhouette pass. Kept pure so the WebGL
// implementation can be validated against deterministic expected masks.
export function dilateMask(mask, width, height, radius = 1) {
  const input = Uint8Array.from(mask);
  const output = new Uint8Array(input.length);
  for (let y = 0; y < height; y += 1) for (let x = 0; x < width; x += 1) {
    let value = 0;
    for (let dy = -radius; dy <= radius && !value; dy += 1) {
      for (let dx = -radius; dx <= radius; dx += 1) {
        const nx = x + dx; const ny = y + dy;
        if (nx >= 0 && nx < width && ny >= 0 && ny < height && input[ny * width + nx]) { value = 255; break; }
      }
    }
    output[y * width + x] = value;
  }
  return output;
}

export const ACTOR_MASK_VERTEX_SHADER = `varying vec2 vUv;void main(){vUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`;
export const ACTOR_MASK_FRAGMENT_SHADER = `uniform sampler2D atlas;varying vec2 vUv;void main(){if(texture2D(atlas,vUv).a<0.01) discard;gl_FragColor=vec4(1.0);}`;
export const ACTOR_MASK_DILATE_FRAGMENT_SHADER = `uniform sampler2D mask;uniform vec2 texel;uniform vec3 outlineColor;varying vec2 vUv;void main(){float c=texture2D(mask,vUv).a;float n=0.;for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++)n=max(n,texture2D(mask,vUv+vec2(float(x),float(y))*texel).a);gl_FragColor=vec4(outlineColor,n*(1.-c));}`;

export function createMaskPass(width = 1, height = 1) {
  return { width, height, vertexShader: ACTOR_MASK_VERTEX_SHADER,
    fragmentShader: ACTOR_MASK_FRAGMENT_SHADER,
    dilationShader: ACTOR_MASK_DILATE_FRAGMENT_SHADER };
}

export function createMaskResources(THREE, width = 1, height = 1) {
  const target = new THREE.WebGLRenderTarget(width, height, { depthBuffer: false, stencilBuffer: false });
  const scene = new THREE.Scene();
  const quadScene = new THREE.Scene();
  const quadCamera = new THREE.Camera();
  const material = new THREE.ShaderMaterial({ uniforms: { atlas: { value: null } }, vertexShader: ACTOR_MASK_VERTEX_SHADER, fragmentShader: ACTOR_MASK_FRAGMENT_SHADER, transparent: true, depthTest: false, depthWrite: false, side: THREE.DoubleSide });
  const dilationMaterial = new THREE.ShaderMaterial({ uniforms: { mask: { value: target.texture }, texel: { value: new THREE.Vector2(1 / width, 1 / height) }, outlineColor: { value: new THREE.Color(0.8, 0.18, 0.04) } }, vertexShader: ACTOR_MASK_VERTEX_SHADER, fragmentShader: ACTOR_MASK_DILATE_FRAGMENT_SHADER, transparent: true, depthTest: false, depthWrite: false, side: THREE.DoubleSide, toneMapped: false, blending: THREE.AdditiveBlending });
  const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), dilationMaterial);
  quadScene.add(quad);
  return { target, scene, quadScene, quadCamera, material, dilationMaterial, quad,
    resize(nextWidth, nextHeight) { target.setSize(nextWidth, nextHeight); dilationMaterial.uniforms.texel.value.set(1 / nextWidth, 1 / nextHeight); },
    dispose() { material.dispose(); dilationMaterial.dispose(); quad.geometry.dispose(); target.dispose(); } };
}
