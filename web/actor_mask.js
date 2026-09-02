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

export const ACTOR_MASK_VERTEX_SHADER = `void main(){gl_Position=vec4(position,1.0);}`;
export const ACTOR_MASK_FRAGMENT_SHADER = `uniform sampler2D atlas; varying vec2 vUv;
void main(){float a=texture2D(atlas,vUv).a;if(a<0.01) discard;gl_FragColor=vec4(1.0);}`;
export const ACTOR_MASK_DILATE_FRAGMENT_SHADER = `uniform sampler2D mask;uniform vec2 texel;
void main(){vec2 u=gl_FragCoord.xy*texel;float c=texture2D(mask,u).a;float n=max(max(texture2D(mask,u+vec2(texel.x,0.)).a,texture2D(mask,u-vec2(texel.x,0.)).a),max(texture2D(mask,u+vec2(0.,texel.y)).a,texture2D(mask,u-vec2(0.,texel.y)).a));gl_FragColor=vec4(vec3(0.02),n*(1.-c));}`;

export function createMaskPass(width = 1, height = 1) {
  return { width, height, vertexShader: ACTOR_MASK_VERTEX_SHADER,
    fragmentShader: ACTOR_MASK_FRAGMENT_SHADER,
    dilationShader: ACTOR_MASK_DILATE_FRAGMENT_SHADER };
}
