// Neutral presentation metadata for sprite actors. This module has no
// dependency on Three.js, Rust state, or the source Spracker application.
export function actorKey(asset) {
    return String(asset || '').replace(/\.glb$/i, '').split('/').pop();
}

export function createActorCatalog(manifest) {
    const actors = manifest?.actors;
    return actors && typeof actors === 'object' ? new Map(Object.entries(actors)) : new Map();
}

/** Build a compact lookup for the offline-exported pose clips. */
export function createPoseCatalog(manifest) {
    const poses = Array.isArray(manifest?.poses) ? manifest.poses : [];
    return new Map(poses
        .filter((pose) => pose && typeof pose.rig === 'string' && typeof pose.clip === 'string')
        .map((pose) => [`${pose.rig}:${pose.clip}`, pose]));
}

const lerp = (a, b, t) => a + (b - a) * t;

function interpolateBone(a, b, t) {
    const position = a.p.map((value, index) => lerp(value, b.p[index], t));
    const quaternion = a.q.map((value, index) => lerp(value, b.q[index], t));
    const length = Math.hypot(...quaternion) || 1;
    return { p: position, q: quaternion.map((value) => value / length) };
}

/** Sample and linearly interpolate one pose clip at an absolute time. */
export function samplePose(catalog, rig, clip, timeMs) {
    const pose = catalog?.get(`${rig}:${clip}`);
    if (!pose || !Array.isArray(pose.frames) || pose.frames.length === 0) return null;
    const duration = Math.max(0, Number(pose.duration_ms) || 0);
    const time = duration > 0 ? ((Number(timeMs) || 0) % duration + duration) % duration : 0;
    const frameRate = Number(pose.fps || 0) || Number(catalog?.fps || 1);
    const position = time * frameRate / 1000;
    const lower = Math.floor(position) % pose.frames.length;
    const upper = (lower + 1) % pose.frames.length;
    const alpha = position - Math.floor(position);
    const first = pose.frames[lower];
    const second = pose.frames[upper];
    return {
        rig,
        clip,
        time_ms: time,
        frame: first.frame,
        next_frame: second.frame,
        alpha,
        bones: first.bones.map((bone, index) => interpolateBone(bone, second.bones[index] || bone, alpha)),
    };
}

export async function loadPoseCatalog(url = './web/sprite_actor_poses.json', fetchImpl = fetch) {
    try {
        const response = await fetchImpl(url);
        return response.ok ? createPoseCatalog(await response.json()) : new Map();
    } catch (_) {
        return new Map();
    }
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
