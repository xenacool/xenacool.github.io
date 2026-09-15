import * as THREE from './vendor/three.module.min.js';
import { GLTFLoader } from './vendor/three/loaders/GLTFLoader.js';
import { clone as cloneSkinned } from './vendor/three/utils/SkeletonUtils.js';
import { actorModelYaw, actorYaw } from './facing.js';

const loader = new GLTFLoader();
const modelPromises = new Map();
const animationBundlePromises = new Map();
const assetStates = new Map();
const RETRY_DELAYS_MS = [250, 1000, 4000];

function reportAssetState(kind, name, state) {
    const detail = { kind, name, ...state };
    assetStates.set(`${kind}:${name}`, detail);
    console.warn(`GLB ${kind} ${name}: ${state.status} (attempt ${state.attempt}/4)${state.error ? `: ${state.error}` : ''}`);
    window.dispatchEvent(new CustomEvent('pystral-glb-asset-state', { detail }));
}

function retryLoad(kind, name, url) {
    const load = async (attempt = 1) => {
        reportAssetState(kind, name, { status: 'loading', attempt, url });
        try {
            const gltf = await loader.loadAsync(url);
            reportAssetState(kind, name, { status: 'ready', attempt, url });
            return gltf;
        } catch (error) {
            const message = error?.message || String(error);
            if (attempt > RETRY_DELAYS_MS.length) {
                reportAssetState(kind, name, { status: 'failed', attempt, url, error: message });
                throw new Error(`${kind} ${name} failed after ${attempt} attempts: ${message}`);
            }
            const delay = RETRY_DELAYS_MS[attempt - 1];
            reportAssetState(kind, name, { status: 'retrying', attempt, url, error: message, retryInMs: delay });
            await new Promise((resolve) => window.setTimeout(resolve, delay));
            return load(attempt + 1);
        }
    };
    return load();
}

export function glbAssetStates() {
    return [...assetStates.values()].map((state) => ({ ...state }));
}

export async function loadModel(manifest, asset) {
    const entry = manifest?.models?.[asset];
    if (!entry?.url) throw new Error(`GLB model is not registered: ${asset}`);
    if (!modelPromises.has(asset)) {
        const promise = retryLoad('model', asset, entry.url).then((gltf) => ({
            scene: gltf.scene,
            animations: gltf.animations || [],
            scale: Number(entry.scale || 1),
        }));
        modelPromises.set(asset, promise);
        promise.catch(() => modelPromises.delete(asset));
    }
    return modelPromises.get(asset);
}

export async function loadAnimationBundle(manifest, bundle) {
    const entry = manifest?.animation_bundles?.[bundle];
    if (!entry?.url) throw new Error(`GLB animation bundle is not registered: ${bundle}`);
    if (!animationBundlePromises.has(bundle)) {
        const promise = retryLoad('animation bundle', bundle, entry.url).then((gltf) => ({
            scene: gltf.scene,
            clips: gltf.animations || [],
            rig: entry.rig,
        }));
        animationBundlePromises.set(bundle, promise);
        promise.catch(() => animationBundlePromises.delete(bundle));
    }
    return animationBundlePromises.get(bundle);
}

function defaultAnimationKey(entity) {
    const state = String(entity.animation_state || 'idle').toLowerCase();
    if (state === 'walk' || state === 'move') return 'movement:Walking_A';
    return 'general:Idle_A';
}

// Every shipped job and animation bundle uses the Medium rig. Three resolves
// track paths against the actor-local SkeletonUtils clone, so cloning a source
// clip creates a deterministic name-to-name retarget without sharing mutable
// AnimationAction state. Some bundles include optional control tracks absent
// from a particular job; Three intentionally ignores those non-skeleton paths.
function retargetCompatibleClip(instance, sourceClip, key) {
    if (!instance || !sourceClip?.tracks?.length) throw new Error(`cannot bind empty clip: ${key}`);
    return sourceClip.clone();
}

function notifyAnimationCompletion(barrier) {
    if (!Number.isSafeInteger(Number(barrier))) return;
    // The browser sends the completion edge only after AnimationMixer reaches
    // the authored clip end. The gate enforces its monotonic ACK watermark.
    // wasm-bindgen represents Rust u64 parameters as JavaScript bigint.
    window.app?.animation_completed(BigInt(barrier));
}

function normalizeAngle(angle) {
    return Math.atan2(Math.sin(angle), Math.cos(angle));
}

export function shortestAngleDelta(from, to) {
    return normalizeAngle(to - from);
}

function easeInOut(progress) {
    const t = Math.min(1, Math.max(0, progress));
    return t * t * (3 - 2 * t);
}

function newOrientation(yaw) {
    return { yaw, tacticalYaw: yaw, targetYaw: yaw, fromYaw: yaw, aimYaw: null, phase: 'tactical', elapsed: 0,
        duration: 0, cue: null, barrier: null, acknowledgementSent: false };
}

function beginTurn(orientation, phase, target, duration, cue = null, barrier = null) {
    orientation.phase = phase;
    orientation.fromYaw = orientation.yaw;
    orientation.targetYaw = target;
    orientation.elapsed = 0;
    orientation.duration = Math.max(0, Number(duration) || 0) / 1000;
    orientation.cue = cue;
    orientation.barrier = barrier;
}

function traceOrientation(record) {
    const trace = window.__pystralPresentationTrace;
    if (!Array.isArray(trace)) return;
    trace.push({ entityId: record.group.userData.entityId, clock: record.orientation.clock,
        phase: record.orientation.phase, cue: record.orientation.cue,
        yaw: record.orientation.yaw });
    if (trace.length > 256) trace.splice(0, trace.length - 256);
}

// Markers point down local -Z while shipped GLBs are authored +Z.  Deriving
// the marker from the actor's current presentation yaw prevents tactical,
// movement, and target-aim paths from drifting into separate conventions.
function syncDirectionMarker(record) {
    const marker = window.__pystralThreeNativeMarkers?.get(String(record.group.userData.entityId));
    if (marker) marker.rotation.y = normalizeAngle(record.orientation.yaw - record.orientation.markerYawOffset);
}

function finishOneShot(animation, cue) {
    if (animation.activeCue !== cue || animation.completedCue === cue) return;
    animation.completedCue = cue;
    animation.oneShot = false;
    const orientation = animation.record?.orientation;
    if (orientation?.cue === cue) {
        beginTurn(orientation, 'returning', orientation.tacticalYaw,
            orientation.turnOutMs, cue, animation.activeBarrier);
        traceOrientation(animation.record);
    }
}

function startOneShotAction(animation, cue, barrier) {
    const clip = animation.pendingCueClip;
    if (!clip || animation.activeCue === cue) return false;
    const nextAction = animation.mixer.clipAction(clip);
    if (animation.activeAction) {
        animation.activeAction.fadeOut(0.12);
        nextAction.reset().fadeIn(0.12);
    } else {
        nextAction.reset();
    }
    nextAction.setLoop(THREE.LoopOnce, Infinity);
    nextAction.clampWhenFinished = true;
    nextAction.play();
    animation.activeClip = clip;
    animation.activeAction = nextAction;
    animation.activeCue = cue;
    animation.oneShot = true;
    animation.activeBarrier = barrier;
    const onFinished = (event) => {
        if (event.action !== nextAction || animation.activeCue !== cue) return;
        finishOneShot(animation, cue);
    };
    animation.mixer.addEventListener('finished', onFinished);
    return true;
}

function disposeModelResources(object) {
    object.traverse((child) => {
        child.geometry?.dispose();
        if (child.material) {
            (Array.isArray(child.material) ? child.material : [child.material])
                .forEach((material) => material.dispose());
        }
    });
}

function disposeRecord(record, mixers, key) {
    record.group.remove(record.instance);
    // Skeleton-safe clones intentionally share immutable geometry, textures,
    // and materials with the cached GLTF prototype. Releasing those resources
    // per entity invalidates siblings using the same job asset.
    if (record.primitive) disposeModelResources(record.primitive);
    const animation = mixers.get(key);
    animation?.mixer.stopAllAction();
    animation?.mixer.uncacheRoot(record.instance);
    mixers.delete(key);
}

function normalizeModel(scene, entry) {
    scene.updateMatrixWorld(true);
    const sourceBounds = new THREE.Box3().setFromObject(scene);
    const sourceSize = sourceBounds.getSize(new THREE.Vector3());
    const sourceHeight = Math.max(sourceSize.y, 0.0001);
    // One world unit is the radius authored by the tactical hex layout. Job
    // actors occupy two of those units by default; entity.scale remains an
    // explicit presentation override for exceptional actors.
    const targetHeight = Number(entry.height || 2);
    const scale = Number(entry.scale || 1) * targetHeight / sourceHeight;
    const center = sourceBounds.getCenter(new THREE.Vector3());
    scene.position.x -= center.x * scale;
    scene.position.y -= sourceBounds.min.y * scale;
    scene.position.z -= center.z * scale;
    scene.scale.setScalar(scale);
    scene.updateMatrixWorld(true);
    const normalizedBounds = new THREE.Box3().setFromObject(scene);
    const normalizedSize = normalizedBounds.getSize(new THREE.Vector3());
    return {
        scene,
        height: normalizedSize.y,
        bounds: {
            min: normalizedBounds.min.toArray(),
            max: normalizedBounds.max.toArray(),
            size: normalizedSize.toArray(),
        },
    };
}

function primitive(asset) {
    const colors = { Rock: 0x777777, FireballOrb: 0xff5a14, FrostBolt: 0x5abfff, SoulOrb: 0xaa46ff };
    const color = colors[asset] || 0xd08040;
    const geometry = asset === 'Arrow'
        ? new THREE.ConeGeometry(0.12, 0.7, 6)
        : new THREE.SphereGeometry(asset === 'Rock' ? 0.35 : 0.2, 12, 8);
    const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({
        color, emissive: asset === 'Rock' ? 0 : color, emissiveIntensity: 0.15,
    }));
    return mesh;
}

export function syncGlbEntities(frame, scene, manifest, meshes, mixers, diagnostics, camera) {
    const seen = new Set();
    for (const entity of frame.entities || []) {
        if (!entity.world_position || !entity.asset) continue;
        const key = String(entity.id);
        seen.add(key);
        let record = meshes.get(key);
        if (record && record.asset !== entity.asset) {
            scene.remove(record.group);
            disposeRecord(record, mixers, key);
            meshes.delete(key);
            record = null;
        }
        if (!record) {
            const yaw = actorYaw(manifest, entity.asset, entity.facing, entity.rotation_y);
            record = { group: new THREE.Group(), loaded: false, asset: entity.asset,
                orientation: newOrientation(Number.isFinite(yaw) ? yaw : 0) };
            record.group.userData.entityId = entity.id;
            scene.add(record.group);
            meshes.set(key, record);
            if (manifest?.models?.[entity.asset]) {
                Promise.all([
                    loadModel(manifest, entity.asset),
                    ...Object.keys(manifest.animation_bundles || {}).map((bundle) =>
                        loadAnimationBundle(manifest, bundle).then((loaded) => [bundle, loaded])),
                ]).then(([model, ...bundles]) => {
                    window.__pystralThreeGlbLoadedModels?.set(entity.asset, model);
                    // Object3D.clone(true) leaves SkinnedMesh.skeleton pointing
                    // at the prototype's bones. SkeletonUtils remaps every
                    // skinned mesh to this actor's cloned bone hierarchy.
                    const instance = cloneSkinned(model.scene);
                    instance.traverse((child) => {
                        if (child.isMesh) { child.castShadow = true; child.receiveShadow = true; }
                    });
                    const normalized = normalizeModel(instance, model);
                    if (meshes.get(key) !== record || record.asset !== entity.asset) {
                        return;
                    }
                    record.instance = instance;
                    record.bounds = normalized.bounds;
                    record.modelHeight = normalized.height;
                    window.__pystralThreeNativeActorContracts?.set(key, {
                        asset: record.asset,
                        bounds: record.bounds,
                        modelHeight: record.modelHeight,
                    });
                    record.group.add(instance);
                    const clips = new Map();
                    for (const [bundle, loaded] of bundles) {
                        if (loaded.rig !== manifest.models[entity.asset].rig) continue;
                        for (const clip of loaded.clips) {
                            const clipKey = `${bundle}:${clip.name}`;
                            clips.set(clipKey, clip);
                        }
                    }
                    const mixer = new THREE.AnimationMixer(instance);
                    mixers.set(key, { mixer, clips, retargeted: new Map(), instance, record });
                    record.loaded = true;
                    window.dispatchEvent(new CustomEvent('pystral-glb-record-attached', {
                        detail: { entityId: entity.id, asset: record.asset },
                    }));
                }).catch((error) => diagnostics(`GLB ${entity.asset}: ${error.message}`));
            } else {
                record.primitive = primitive(entity.asset);
                record.group.add(record.primitive);
                record.loaded = true;
            }
        }
        record.group.position.fromArray(entity.world_position);
        const offset = entity.camera_offset;
        if (camera && Array.isArray(offset) && offset.length === 3) {
            const e = camera.matrixWorld.elements;
            record.group.position.x += e[0] * offset[0] + e[4] * offset[1] - e[8] * offset[2];
            record.group.position.y += e[1] * offset[0] + e[5] * offset[1] - e[9] * offset[2];
            record.group.position.z += e[2] * offset[0] + e[6] * offset[1] - e[10] * offset[2];
        }
        record.group.scale.setScalar(Number(entity.scale || 1));
        record.group.renderOrder = Number(entity.render_order || 0) * 1000;
        const orientation = record.orientation;
        orientation.clock = Number(frame.presentation_debug?.presentation_clock ?? frame.tick ?? 0);
        const tacticalYaw = actorYaw(manifest, entity.asset, entity.facing, entity.rotation_y);
        orientation.tacticalYaw = tacticalYaw;
        orientation.turnInMs = Number(frame.presentation?.attack_turn_in_ms ?? 150);
        orientation.turnOutMs = Number(frame.presentation?.attack_turn_out_ms ?? 180);
        orientation.markerYawOffset = actorModelYaw(manifest, entity.asset, entity.rotation_y);
        const aim = Number(entity.presentation_aim_yaw);
        orientation.aimYaw = Number.isFinite(aim)
            ? aim + actorModelYaw(manifest, entity.asset, entity.rotation_y)
            : null;
        if (orientation.phase === 'tactical' && Math.abs(shortestAngleDelta(orientation.targetYaw ?? orientation.yaw, tacticalYaw)) > 1e-6) {
            beginTurn(orientation, 'tactical', tacticalYaw, frame.presentation?.animation_crossfade_ms ?? 120);
        }
        record.group.rotation.y = orientation.yaw;
        syncDirectionMarker(record);
        const animation = mixers.get(key);
        if (animation) {
            const cue = entity.animation_cue == null ? null : Number(entity.animation_cue);
            const unconsumedCue = cue !== null && animation.completedCue !== cue;
            if (unconsumedCue && orientation.cue !== cue) {
                if (orientation.aimYaw === null) {
                    // A malformed self-targeting cue still preserves the
                    // barrier contract: there is no turn-in phase to await.
                    orientation.cue = cue;
                    orientation.barrier = Number(entity.animation_barrier);
                    orientation.phase = 'attacking';
                } else {
                    beginTurn(orientation, 'aiming', orientation.aimYaw,
                        orientation.turnInMs, cue, Number(entity.animation_barrier));
                    traceOrientation(record);
                }
            }
            const requested = unconsumedCue && orientation.phase === 'attacking'
                ? String(entity.animation_cue_clip || entity.animation_clip || defaultAnimationKey(entity))
                : String(entity.animation_clip || defaultAnimationKey(entity));
            if (unconsumedCue) {
                const cueKey = String(entity.animation_cue_clip || entity.animation_clip || defaultAnimationKey(entity));
                const cueSource = animation.clips.get(cueKey);
                if (!animation.retargeted.get(cueKey) && cueSource) {
                    try {
                        animation.retargeted.set(cueKey,
                            retargetCompatibleClip(animation.instance, cueSource, cueKey));
                    } catch (error) {
                        diagnostics(`GLB ${record.asset}: ${error.message}`);
                    }
                }
                animation.pendingCueClip = animation.retargeted.get(cueKey);
            }
            const sourceClip = animation.clips.get(requested);
            let clip = animation.retargeted.get(requested);
            if (!clip && sourceClip) {
                try {
                    clip = retargetCompatibleClip(animation.instance, sourceClip, requested);
                    animation.retargeted.set(requested, clip);
                } catch (error) {
                    diagnostics(`GLB ${record.asset}: ${error.message}`);
                }
            }
            if (clip) {
                // Cues remain in immutable history after completion. Only an
                // unconsumed cue may configure a one-shot; otherwise the
                // restored idle/walk clip would clamp at its first end.
                const oneShot = unconsumedCue && orientation.phase === 'attacking';
                const newCue = oneShot && animation.activeCue !== cue;
                if (animation.activeClip !== clip || newCue) {
                    if (oneShot) {
                        startOneShotAction(animation, cue, Number(entity.animation_barrier));
                        continue;
                    }
                    const nextAction = animation.mixer.clipAction(clip);
                    if (animation.activeAction) {
                        animation.activeAction.fadeOut(0.12);
                        nextAction.reset().fadeIn(0.12);
                    } else {
                        nextAction.reset();
                    }
                    nextAction.setLoop(THREE.LoopRepeat, Infinity);
                    nextAction.clampWhenFinished = false;
                    nextAction.play();
                    animation.activeClip = clip;
                    animation.activeAction = nextAction;
                    animation.activeCue = null;
                    animation.oneShot = false;
                }
                // Looping clips own their presentation clock. Runtime frames
                // deliberately carry no per-actor animation clock for normal
                // GLB units, so seeking each frame would pin both idle and
                // walking to their first pose. One-shot cues are separately
                // reset when their cue identity changes.
            } else if (requested) {
                diagnostics(`GLB ${record.asset}: animation clip is not available: ${requested}`);
            }
        }
    }
    for (const [key, record] of meshes) {
        if (!seen.has(key)) {
            scene.remove(record.group);
            disposeRecord(record, mixers, key);
            meshes.delete(key);
        }
    }
}

export function advanceGlbAnimations(mixers, deltaSeconds) {
    for (const animation of mixers.values()) {
        const orientation = animation.record?.orientation;
        if (orientation) {
            const duration = orientation.duration;
            orientation.elapsed += Math.max(0, deltaSeconds);
            const progress = duration <= 0 ? 1 : Math.min(1, orientation.elapsed / duration);
            orientation.yaw = normalizeAngle(orientation.fromYaw
                + shortestAngleDelta(orientation.fromYaw, orientation.targetYaw) * easeInOut(progress));
            animation.record.group.rotation.y = orientation.yaw;
            syncDirectionMarker(animation.record);
            if (progress === 1) {
                if (orientation.phase === 'aiming') {
                    orientation.phase = 'attacking';
                    // The cue clip was prepared while turning so this edge is
                    // independent of arrival of another render frame.
                    startOneShotAction(animation, orientation.cue, orientation.barrier);
                    traceOrientation(animation.record);
                } else if (orientation.phase === 'returning' && !orientation.acknowledgementSent) {
                    orientation.phase = 'tactical';
                    orientation.acknowledgementSent = true;
                    notifyAnimationCompletion(orientation.barrier);
                    traceOrientation(animation.record);
                } else if (orientation.phase === 'tactical') {
                    // A settled tactical turn must become a fixed point.
                    // Keeping the old source and duration here would replay
                    // the same turn on every render tick.
                    orientation.yaw = orientation.targetYaw;
                    orientation.fromYaw = orientation.targetYaw;
                    orientation.duration = 0;
                }
            }
        }
        const priorTime = animation.activeAction?.time ?? 0;
        animation.mixer.update(Math.max(0, deltaSeconds));
        if (!animation.oneShot && animation.activeClip?.duration > 0
            && (animation.activeAction?.time ?? 0) < priorTime) {
            animation.loopCount = (animation.loopCount || 0) + 1;
        }
        if (animation.oneShot && animation.activeAction?.time >= animation.activeClip?.duration) {
            finishOneShot(animation, animation.activeCue);
        }
    }
}
