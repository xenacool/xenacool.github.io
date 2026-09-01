// Neutral presentation metadata for sprite actors. This module has no
// dependency on Three.js, Rust state, or the source Spracker application.
export function actorKey(asset) {
    return String(asset || '').replace(/\.glb$/i, '').split('/').pop();
}

export function createActorCatalog(manifest) {
    const actors = manifest?.actors;
    return actors && typeof actors === 'object' ? new Map(Object.entries(actors)) : new Map();
}

export function sliceInfluence(actor, sliceIndex) {
    const samples = actor?.slice_influences;
    if (!Array.isArray(samples) || samples.length === 0) return null;
    const stride = Math.max(1, Number(actor.slice_stride) || 1);
    const index = Math.max(0, Math.min(samples.length - 1,
        Math.round((Number(sliceIndex) || 0) / stride)));
    return samples[index]?.bones || null;
}

export async function loadActorCatalog(url = './web/sprite_actor_manifest.json', fetchImpl = fetch) {
    try {
        const response = await fetchImpl(url);
        return response.ok ? createActorCatalog(await response.json()) : new Map();
    } catch (_) {
        return new Map();
    }
}
