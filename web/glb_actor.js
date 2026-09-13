import * as THREE from './vendor/three.module.min.js';
import { GLTFLoader } from './vendor/three/loaders/GLTFLoader.js';
import { clone as cloneSkinned } from './vendor/three/utils/SkeletonUtils.js';

const loader = new GLTFLoader();
const modelPromises = new Map();

export async function loadModel(manifest, asset) {
    const entry = manifest?.models?.[asset];
    if (!entry?.url) throw new Error(`GLB model is not registered: ${asset}`);
    if (!modelPromises.has(asset)) {
        modelPromises.set(asset, loader.loadAsync(entry.url).then((gltf) => ({
            scene: gltf.scene,
            animations: gltf.animations || [],
            scale: Number(entry.scale || 1),
        })));
    }
    return modelPromises.get(asset);
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
            record = { group: new THREE.Group(), loaded: false, asset: entity.asset };
            record.group.userData.entityId = entity.id;
            scene.add(record.group);
            meshes.set(key, record);
            if (manifest?.models?.[entity.asset]) {
                loadModel(manifest, entity.asset).then((model) => {
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
                    const mixer = new THREE.AnimationMixer(instance);
                    mixers.set(key, { mixer, clips: model.animations });
                    record.loaded = true;
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
        const facing = { north: 0, northeast: 1, southeast: 2, south: 3, southwest: 4, northwest: 5 };
        record.group.rotation.y = (facing[String(entity.facing || '').toLowerCase()] || 0) * Math.PI / 3
            + Number(entity.rotation_y || 0);
        const animation = mixers.get(key);
        if (animation) {
            const requested = entity.animation_clip || entity.animation_state || 'idle';
            const clip = animation.clips.find((candidate) => candidate.name === requested)
                || animation.clips.find((candidate) => candidate.name.toLowerCase() === requested.toLowerCase());
            if (clip) {
                if (animation.activeClip !== clip) {
                    const nextAction = animation.mixer.clipAction(clip);
                    if (animation.activeAction) {
                        animation.activeAction.fadeOut(0.12);
                        nextAction.reset().fadeIn(0.12).play();
                    } else {
                        nextAction.reset().play();
                    }
                    animation.activeClip = clip;
                    animation.activeAction = nextAction;
                }
                animation.mixer.setTime(Math.max(0, Number(entity.animation_time_ms || 0)) / 1000);
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
