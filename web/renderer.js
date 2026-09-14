// Three.js r178 presentation compositor.
// Rust publishes authoritative simulation presentation data; Three.js owns the
// visible WebGL2 canvas.
import * as THREE from './vendor/three.module.min.js';
import { advanceGlbAnimations, loadAnimationBundle, loadModel, syncGlbEntities } from './glb_actor.js';
import { facingYaw, HEX_DIRECTION_OFFSET } from './facing.js';

const PRESENTATION_MOTION = Object.freeze({
    idleAmplitude: 0.025,
    walkAmplitude: 0.06,
    hitRecoil: 0.08,
});
export function hexCenter(q, r, pointy, sizeX, sizeZ) {
    return pointy
        ? [Math.sqrt(3) * (q + r / 2) * sizeX, 1.5 * r * sizeZ]
        : [1.5 * q * sizeX, Math.sqrt(3) * (r + q / 2) * sizeZ];
}

export function cubicBezierEase(t) {
    const clamped = Math.max(0, Math.min(1, Number(t) || 0));
    return clamped * clamped * (3 - 2 * clamped);
}

export function presentationOffset(entity, reducedMotion = false) {
    if (reducedMotion) return [0, 0, 0];
    const state = String(entity.animation_state || 'idle').toLowerCase();
    const time = Math.max(0, Number(entity.animation_time_ms) || 0) / 1000;
    if (state === 'idle') {
        return [0, Math.sin(time * Math.PI * 2) * PRESENTATION_MOTION.idleAmplitude, 0];
    }
    if (state === 'walk' || state === 'move') {
        const phase = (time % 0.5) / 0.5;
        const arc = cubicBezierEase(phase < 0.5 ? phase * 2 : (1 - phase) * 2);
        return [0, arc * PRESENTATION_MOTION.walkAmplitude, 0];
    }
    if (state === 'hit' || state === 'hurt') {
        const recoil = Math.max(0, 1 - cubicBezierEase(Math.min(1, time / 0.25)));
        return [0, recoil * PRESENTATION_MOTION.hitRecoil, 0];
    }
    return [0, 0, 0];
}


export function createNativePresentation(canvas) {
    window.__pystralThreeStaticMap = null;
    window.__pystralThreeStaticMaterials = null;
    let renderer;
    try {
        renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
    } catch (error) {
        console.warn('Three.js WebGL2 renderer unavailable.', error);
        return { available: false, dispose() {} };
    }

    renderer.setPixelRatio(1);
    renderer.autoClear = false;
    renderer.setClearColor(0x1a1a1a, 1);
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    const scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight(0xffffff, 0x334455, 1.2));
    const keyLight = new THREE.DirectionalLight(0xffffff, 1.0);
    keyLight.position.set(4, 8, 6);
    keyLight.castShadow = true;
    keyLight.shadow.mapSize.set(1024, 1024);
    keyLight.shadow.camera.near = 0.1;
    keyLight.shadow.camera.far = 40;
    scene.add(keyLight);
    const camera = new THREE.Camera();
    const actorMask = null;
    window.__pystralThreeActorMask = actorMask;
    let glbManifest = null;
    const nativeMeshes = new Map();
    window.__pystralThreeNativeActorContracts = new Map();
    window.__pystralThreeGlbLoadedModels = new Map();
    const teamMarkers = new Map();
    const waypointMarkers = new Map();
    const mixers = new Map();
    window.__pystralThreeMixers = mixers;
    window.__pystralThreeAdvanceAnimations = (seconds) => advanceGlbAnimations(mixers, seconds);
    window.__pystralThreePoseProfile = () => ({ ready: glbManifest !== null });
    window.__pystralThreeNativeMarkers = teamMarkers;
    window.__pystralThreeNativeWaypointMarkers = waypointMarkers;
    const teamMarkerGeometry = new THREE.RingGeometry(0.32, 0.40, 16);
    const facingMarkerGeometry = new THREE.ConeGeometry(0.16, 0.42, 3);
    window.__pystralThreeTeamMarkerGeometry = teamMarkerGeometry;
    window.__pystralThreeFacingMarkerGeometry = facingMarkerGeometry;
    window.__pystralThreeNativeScene = scene;
        window.__pystralThreeNativeCamera = camera;
        window.__pystralThreeNativeMeshes = nativeMeshes;
    window.__pystralThreeNativeProfile = { entities: 0, mapTiles: 0 };
    window.__pystralThreeMaskEnabled = true;
    const profile = {
        // 60 Hz is the highest stable target across supported WebGL2 browsers;
        // requestAnimationFrame naturally adapts down on slower displays.
        targetFps: 60,
        frames: 0,
        resizeCalls: 0,
        totalRenderMs: 0,
        lastFrameAt: 0,
        lastFrameIntervalMs: 0,
        droppedFrames: 0,
        frameIntervalsMs: [],
        frameSamples: 0,
        appliedFrameSamples: 0,
        positionSamples: 0,
        nativeGlbReady: false,
        nativeGlbModels: 0,
        nativeMeshCount: 0,
        nativeGpu: { drawCalls: 0, triangles: 0, textures: 0, glErrors: 0 },
        fps: {
            simulation: { static: createFpsBucket(), rotating: createFpsBucket() },
            acquiescent: { static: createFpsBucket(), rotating: createFpsBucket() },
        },
    };
    let previousCameraSignature = null;
    let cameraMotionPending = false;
    let cameraMotionUntil = 0;
    let lastTelemetryEventAt = 0;
    let lastFpsBucketKey = null;
    window.__pystralThreeProfile = () => ({ ...profile });
    window.__pystralThreeFpsProfile = () => snapshotFps(profile);
    window.__pystralThreeGlbProfile = () => ({ ready: profile.nativeGlbReady, models: profile.nativeGlbModels });
    // The camera controller can mark a requested transition immediately;
    // frame ingress also marks changes observed in authoritative camera data.
    window.__pystralThreeMarkCameraMotion = () => {
        cameraMotionPending = true;
        cameraMotionUntil = performance.now() + 750;
    };

    const resize = () => {
        const width = Math.max(1, Math.floor(canvas.clientWidth));
        const height = Math.max(1, Math.floor(canvas.clientHeight));
        if (canvas.width !== width || canvas.height !== height) {
            renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
            profile.resizeCalls += 1;
        }
    };
    const frameListener = (event) => {
        // Rust has already resolved animation state and slice ownership.
        // This ingress only retains presentation data; the scene-native pass
        // may resolve static atlas UVs for the supplied slice indices, never
        // gameplay or animation decisions.
        const frame = event.detail;
        profile.frameSamples += 1;
        profile.appliedFrameSamples += 1;
        if (Array.isArray(frame.entities) && frame.entities.length > 0
            && frame.entities.every((entity) => Array.isArray(entity.world_position)
                && entity.world_position.length === 3)) {
            profile.positionSamples += 1;
        }
        const nextCameraSignature = cameraSignatureFor(frame);
        if (previousCameraSignature !== null
            && nextCameraSignature !== null
            && nextCameraSignature !== previousCameraSignature) {
            cameraMotionPending = true;
            cameraMotionUntil = performance.now() + 750;
        }
        previousCameraSignature = nextCameraSignature;
        if (frame.map) window.__pystralThreeStaticMap = frame.map;
        if (frame.materials && Object.keys(frame.materials).length > 0) {
            window.__pystralThreeStaticMaterials = frame.materials;
        }
        window.__pystralThreeFrame = {
            ...frame,
            map: window.__pystralThreeStaticMap || null,
            materials: window.__pystralThreeStaticMaterials || {},
        };
        applyNativeFrame(window.__pystralThreeFrame);
        window.__pystralRenderFramePending = false;
    };
    window.addEventListener('pystral-render-frame', frameListener);
    fetch('./web/glb_manifest.json')
        .then((response) => response.ok ? response.json() : Promise.reject(response.status))
        .then((manifest) => {
            glbManifest = manifest;
            window.__pystralThreeGlbManifest = manifest;
            // Use the same production loader for deterministic asset checks
            // without coupling them to the lifetime of an entity in a live
            // authoritative frame.
            window.__pystralThreeLoadGlbModels = (assets) => Promise.all(
                [...new Set(assets || [])].map((asset) => loadModel(manifest, asset).then((model) => {
                    window.__pystralThreeGlbLoadedModels.set(asset, model);
                    return model;
                }))
            );
            window.__pystralThreeLoadAnimationBundles = (bundles) => Promise.all(
                [...new Set(bundles || [])].map((bundle) => loadAnimationBundle(manifest, bundle))
            );
            profile.nativeGlbReady = true;
            profile.nativeGlbModels = Object.keys(manifest.models || {}).length;
            window.dispatchEvent(new CustomEvent('pystral-three-glb-ready'));
            if (window.__pystralThreeFrame) applyNativeFrame(window.__pystralThreeFrame);
        })
        .catch((error) => console.warn('Native GLB manifest unavailable:', error));
    let active = true;
    const render = () => {
        if (!active) return;
        const frameStart = performance.now();
        const frameIntervalMs = profile.lastFrameAt > 0
            ? frameStart - profile.lastFrameAt
            : 0;
        if (profile.lastFrameAt > 0) {
            profile.lastFrameIntervalMs = frameIntervalMs;
        }
        profile.lastFrameAt = frameStart;
        advanceGlbAnimations(mixers, Math.min(frameIntervalMs || 0, 100) / 1000);
        resize();
        renderer.setRenderTarget(null);
        renderer.setClearColor(0x1a1a1a, 1);
        renderer.clear(true, true, true);
        if (actorMask) {
            actorMask.resize(Math.max(1, Math.floor(canvas.width / 2)), Math.max(1, Math.floor(canvas.height / 2)));
            actorMask.material.uniforms.atlas.value = atlasTexture;
            renderer.setRenderTarget(actorMask.target);
            renderer.setClearColor(0x000000, 0);
            renderer.clear();
            renderer.render(actorMask.scene, camera);
            renderer.setRenderTarget(null);
            renderer.setClearColor(0x1a1a1a, 1);
            window.__pystralThreeMaskProfile = {
                meshes: actorMaskMeshes.size,
                target: [actorMask.target.width, actorMask.target.height],
            };
        }
        const rotating = cameraMotionPending || frameStart < cameraMotionUntil;
        cameraMotionPending = false;
        const phase = phaseForStatus(window.__pystralWorkerStatus);
        renderer.render(scene, camera);
        if (actorMask) {
            renderer.render(actorMask.quadScene, actorMask.quadCamera);
        }
        {
            const gl = renderer.getContext();
            profile.nativeGpu = {
                drawCalls: renderer.info.render.calls,
                triangles: renderer.info.render.triangles,
                textures: renderer.info.memory.textures,
                // Sampling after each frame keeps browser diagnostics visible
                // without introducing a second instrumentation render pass.
                glErrors: gl.getError() === gl.NO_ERROR ? 0 : 1,
            };
            window.__pystralThreeNativeProfile = {
                ...(window.__pystralThreeNativeProfile || {}),
                nativeGpu: profile.nativeGpu,
            };
        }
        const renderMs = performance.now() - frameStart;
        if (phase) {
            const motion = rotating ? 'rotating' : 'static';
            const bucketKey = `${phase}/${motion}`;
            const bucket = profile.fps[phase][motion];
            if (bucketKey !== lastFpsBucketKey) bucket.lastFrameAt = 0;
            recordFpsSample(bucket, frameStart, renderMs, profile.targetFps);
            lastFpsBucketKey = bucketKey;
        }
        if (frameIntervalMs > 0) {
            const expectedMs = 1000 / profile.targetFps;
            profile.frameIntervalsMs.push(frameIntervalMs);
            if (profile.frameIntervalsMs.length > 240) profile.frameIntervalsMs.shift();
            if (frameIntervalMs > expectedMs * 1.5) {
                profile.droppedFrames += Math.max(1, Math.round(frameIntervalMs / expectedMs) - 1);
            }
        }
        profile.frames += 1;
        profile.totalRenderMs += renderMs;
        if (frameStart - lastTelemetryEventAt >= 500) {
            lastTelemetryEventAt = frameStart;
            window.dispatchEvent(new CustomEvent('pystral-three-fps', {
                detail: snapshotFps(profile),
            }));
        }
        requestAnimationFrame(render);
    };
    requestAnimationFrame(render);

    return {
        available: true,
        setActorCatalog(catalog) {
            // Metadata is retained for the next presentation effect pass. It
            // deliberately does not replay or mutate the authoritative frame.
            window.__pystralThreeActorCatalog = catalog instanceof Map ? catalog : new Map();
        },
        setPoseCatalog(catalog) {
            if (window.__pystralThreeFrame) applyNativeFrame(window.__pystralThreeFrame);
        },
        dispose() {
            active = false;
            window.removeEventListener('pystral-render-frame', frameListener);
            delete window.__pystralThreeMarkCameraMotion;
            delete window.__pystralThreeGlbProfile;
            delete window.__pystralThreePoseProfile;
            delete window.__pystralThreeNativeScene;
            delete window.__pystralThreeNativeCamera;
            delete window.__pystralThreeNativeMeshes;
            delete window.__pystralThreeNativeActorContracts;
            delete window.__pystralThreeGlbLoadedModels;
            delete window.__pystralThreeNativeMarkers;
            delete window.__pystralThreeNativeWaypointMarkers;
            delete window.__pystralThreeActorMask;
            delete window.__pystralThreeMaskProfile;
            delete window.__pystralThreeTeamMarkerGeometry;
            delete window.__pystralThreeFacingMarkerGeometry;
            delete window.__pystralThreeGlbManifest;
            delete window.__pystralThreeLoadGlbModels;
            delete window.__pystralThreeLoadAnimationBundles;
            delete window.__pystralThreeAdvanceAnimations;
            delete window.__pystralThreeActorCatalog;
            actorMaskMeshes.forEach((mesh) => mesh.material.dispose());
            actorMaskMeshes.clear();
            actorMask?.dispose();
            window.__pystralThreeNativeHexGeometry?.dispose();
            nativeMeshes.forEach((record) => {
                scene.remove(record.group);
            });
            (window.__pystralThreeNativeTileMeshes || new Map()).forEach((mesh) => {
                mesh.material.dispose();
            });
            teamMarkers.forEach((marker) => {
                scene.remove(marker);
                marker.traverse((child) => child.material?.dispose());
            });
            waypointMarkers.forEach((marker) => {
                scene.remove(marker);
                marker.traverse((child) => child.material?.dispose());
            });
            teamMarkerGeometry.dispose();
            facingMarkerGeometry.dispose();
            if (window.__pystralThreeCompass) {
                window.__pystralThreeCompass.traverse((child) => {
                    child.material?.map?.dispose();
                    child.material?.dispose();
                    child.geometry?.dispose();
                });
                scene.remove(window.__pystralThreeCompass);
                delete window.__pystralThreeCompass;
            }
            delete window.__pystralThreeNativeHexGeometry;
            delete window.__pystralThreeNativeTileMeshes;
            renderer.dispose();
        },
    };
}
function syncNativeWaypoints(frame) {
    const waypointMarkers = window.__pystralThreeNativeWaypointMarkers;
    const teamMarkerGeometry = window.__pystralThreeTeamMarkerGeometry;
    if (!waypointMarkers || !teamMarkerGeometry) return;
    const seenWaypointMarkers = new Set();
    const presentation = frame.presentation || {};
    const syncWaypoint = (kind, waypoint, color, opacity, order) => {
        if (!waypoint?.world_position) return;
        const key = `${kind}:${waypoint.q}:${waypoint.r}:${waypoint.layer}`;
        let marker = waypointMarkers.get(key);
        if (!marker) {
            marker = new THREE.Group();
            const ring = new THREE.Mesh(teamMarkerGeometry, new THREE.MeshBasicMaterial({
                color: new THREE.Color(...color), transparent: true, opacity,
                blending: THREE.AdditiveBlending, depthTest: false, depthWrite: false,
                side: THREE.DoubleSide,
            }));
            ring.rotation.x = -Math.PI / 2;
            marker.add(ring);
            window.__pystralThreeNativeScene.add(marker);
            waypointMarkers.set(key, marker);
        }
        marker.position.fromArray(waypoint.world_position);
        marker.position.y += 0.12;
        marker.scale.setScalar((Number(presentation.marker_scale) || 1) * (kind === 'selected' ? 1.25 : 0.9));
        marker.traverse((child) => {
            if (child.material) {
                child.material.color.setRGB(...color);
                child.material.opacity = opacity;
            }
        });
        marker.renderOrder = 900000 + order;
        seenWaypointMarkers.add(key);
    };
    const preview = frame.waypoint_preview;
    (preview?.reachable || []).forEach((waypoint, index) => syncWaypoint(
        'reachable', waypoint, presentation.reachable_color || [0.25, 0.65, 1],
        Number(presentation.reachable_opacity ?? 0.28), index,
    ));
    (preview?.path || []).forEach((waypoint, index) => syncWaypoint(
        'path', waypoint, presentation.path_color || [0.35, 0.9, 1],
        Number(presentation.path_opacity ?? 0.62), 10000 + index,
    ));
    if (preview?.selected_destination) syncWaypoint(
        'selected', preview.selected_destination,
        presentation.selected_color || [1, 0.9, 0.25],
        Number(presentation.selected_opacity ?? 0.95), 20000,
    );
    waypointMarkers.forEach((marker, key) => {
        if (!seenWaypointMarkers.has(key)) {
            window.__pystralThreeNativeScene.remove(marker);
            marker.traverse((child) => child.material?.dispose());
            waypointMarkers.delete(key);
        }
    });
}

function syncNativeIndicators(frame) {
    const markers = window.__pystralThreeNativeMarkers;
    const ringGeometry = window.__pystralThreeTeamMarkerGeometry;
    const facingGeometry = window.__pystralThreeFacingMarkerGeometry;
    const scene = window.__pystralThreeNativeScene;
    if (!markers || !ringGeometry || !facingGeometry || !scene) return;
    const seen = new Set();
    (frame.entities || []).forEach((entity) => {
        const indicator = entity.indicator;
        if (!indicator || !entity.world_position) return;
        if (indicator.kind === 'facing' && (!entity.asset || !isActorEntity(entity))) return;
        const id = String(entity.id);
        const color = Array.isArray(indicator.color)
            ? new THREE.Color(...indicator.color) : new THREE.Color(indicator.color || 0xffffff);
        const state = String(indicator.state || 'committed');
        let marker = markers.get(id);
        if (!marker) {
            marker = new THREE.Group();
            const ring = new THREE.Mesh(ringGeometry, new THREE.MeshBasicMaterial({
                color, transparent: true, opacity: state === 'hover' ? 0.45 : 0.85,
                blending: THREE.AdditiveBlending, depthTest: false, depthWrite: false,
                side: THREE.DoubleSide,
            }));
            ring.rotation.x = -Math.PI / 2;
            marker.add(ring);
            if (indicator.kind === 'facing') {
                const arrow = new THREE.Mesh(facingGeometry, ring.material.clone());
                arrow.rotation.x = -Math.PI / 2;
                arrow.position.z = -0.46;
                marker.add(arrow);
            }
            scene.add(marker);
            markers.set(id, marker);
        }
        marker.traverse((child) => {
            if (child.material) {
                child.material.color.copy(color);
                child.material.opacity = state === 'hover' ? 0.45 : 0.85;
            }
        });
        marker.position.fromArray(entity.world_position);
        marker.position.y += 0.16;
        marker.scale.setScalar((Number(entity.scale) || 1) * 1.15);
        marker.rotation.y = facingYaw(indicator.direction);
        marker.renderOrder = Number(entity.render_order || 0) * 1000 + 100000;
        seen.add(id);
    });
    markers.forEach((marker, id) => {
        if (!seen.has(id)) {
            scene.remove(marker);
            marker.traverse((child) => child.material?.dispose());
            markers.delete(id);
        }
    });
}

function applyNativeFrame(frame) {
    const scene = window.__pystralThreeNativeScene;
    const camera = window.__pystralThreeNativeCamera;
    if (!scene || !camera) return;
    if (frame.camera_pose) {
        camera.matrixWorldInverse.fromArray(frame.camera_pose.view);
        camera.projectionMatrix.fromArray(frame.camera_pose.projection);
        camera.matrixWorld.copy(camera.matrixWorldInverse).invert();
        camera.matrixAutoUpdate = false;
        camera.matrixWorld.decompose(camera.position, camera.quaternion, camera.scale);
    }
    applyNativeMap(frame, scene);
    syncNativeWaypoints(frame);
    syncNativeIndicators(frame);
    const manifest = window.__pystralThreeGlbManifest;
    if (manifest) {
        syncGlbEntities(frame, scene, manifest, window.__pystralThreeNativeMeshes,
            window.__pystralThreeMixers || new Map(),
            (message) => console.warn(message), camera);
        window.__pystralThreeNativeProfile = {
            ...(window.__pystralThreeNativeProfile || {}),
            entities: (frame.entities || []).filter((entity) => entity.world_position).length,
            mapTiles: frame.map?.tiles?.length || 0,
            nativeMeshCount: window.__pystralThreeNativeMeshes?.size || 0,
        };
        return;
    }
    // Frames may arrive before the manifest. The manifest-ready callback
    // replays the retained frame, so no fallback scene is required.
    return;
    /* legacy atlas renderer removed */
    const meshes = window.__pystralThreeNativeMeshes;
    const teamMarkers = window.__pystralThreeNativeMarkers;
    const teamMarkerGeometry = window.__pystralThreeTeamMarkerGeometry;
    const facingMarkerGeometry = window.__pystralThreeFacingMarkerGeometry;
    const actorMask = window.__pystralThreeActorMask;
    const reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches === true;
    const seen = new Set();
    const seenMarkers = new Set();
    (frame.entities || []).forEach((entity) => {
        if (!entity.world_position || !entity.asset) return;
        // Pose identity is authored by the runtime frame. Keep it explicit at
        // the compositor boundary; never infer a clip from a generic state.
        const rig = entity.rig || null;
        const animationClip = entity.animation_clip || null;
        const sliceIndices = Array.isArray(entity.slice_indices) && entity.slice_indices.length > 0
            ? entity.slice_indices
            : [entity.selected_slice_index ?? 0];
        const sliceCount = sliceIndices.length;
        sliceIndices.forEach((sliceIndex, stackIndex) => {
            const region = resolveAtlasRegion(atlas, entity.asset, sliceIndex);
            if (!region) return;
            const key = `${entity.id}:${stackIndex}`;
            let mesh = meshes.get(key);
            if (!mesh) {
                const entityMaterial = new THREE.MeshBasicMaterial({
                    map: texture,
                    transparent: true,
                    alphaTest: 0.01,
                    depthWrite: entity.kind !== 'projectile',
                    blending: entity.kind === 'projectile' ? THREE.AdditiveBlending : THREE.NormalBlending,
                    // Facing is gameplay-authored and may point away from
                    // the current camera. Spritestack cutouts remain visible
                    // from either side while retaining that orientation.
                    side: THREE.DoubleSide,
                });
                mesh = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), entityMaterial);
                mesh.castShadow = true;
                scene.add(mesh);
                meshes.set(key, mesh);
            }
            const regionKey = `${region.x}:${region.y}:${region.width}:${region.height}`;
            if (mesh.__pystralAtlasRegionKey !== regionKey) {
                const uv = mesh.geometry.getAttribute('uv');
                uv.setXY(0, region.u0, region.v0);
                uv.setXY(1, region.u1, region.v0);
                uv.setXY(2, region.u0, region.v1);
                uv.setXY(3, region.u1, region.v1);
                uv.needsUpdate = true;
                mesh.__pystralAtlasRegionKey = regionKey;
            }
            const scale = entity.scale || 1;
            const teamRoleColor = entity.kind === 'projectile' ? '#7DEBFF' : '#FFFFFF';
            if (mesh.__pystralTeamRoleColor !== teamRoleColor) {
                const teamColor = new THREE.Color(teamRoleColor);
                if (entity.kind === 'projectile') mesh.material.color.copy(teamColor);
                else mesh.material.color.set('#FFFFFF');
                const outlineShader = mesh.material.userData.outlineShader;
                if (outlineShader?.uniforms.outlineColor) {
                    outlineShader.uniforms.outlineColor.value = [
                        teamColor.r * 0.35, teamColor.g * 0.35, teamColor.b * 0.35,
                    ];
                }
                mesh.__pystralTeamRoleColor = teamRoleColor;
            }
            const dimensions = Array.isArray(entity.stack_dimensions)
                ? entity.stack_dimensions : [1, 0, 1];
            const unitHeight = Number(entity.unit_height) || 0;
            const stackSpan = Math.min(
                Number(dimensions[1]) || 0,
                unitHeight > 0 ? unitHeight * 0.95 : Number(dimensions[1]) || 0,
            );
            const spacing = Number(entity.stack_spacing) || 0;
            mesh.position.fromArray(entity.world_position);
            addCameraRelativeOffset(mesh.position, camera, entity.camera_offset);
            const motion = presentationOffset(entity, reducedMotion);
            mesh.position.x += motion[0] * scale;
            mesh.position.y += motion[1] * scale;
            mesh.position.z += motion[2] * scale;
            // Kept only in the unreachable compatibility block below while
            // old recordings are retired; live presentation uses GLB groups.
            const effectiveSpacing = spacing > 0 && sliceCount > 1
                ? Math.min(spacing, stackSpan / (sliceCount - 1))
                : 0;
            const layerOffset = effectiveSpacing > 0
                ? stackIndex * effectiveSpacing
                : (sliceCount > 1 ? stackIndex / (sliceCount - 1) * stackSpan : 0);
            mesh.position.y += layerOffset * scale;
            mesh.scale.set(
                (Number(dimensions[0]) || 1) * scale,
                (Number(dimensions[2]) || 1) * scale,
                1,
            );
            mesh.renderOrder = Number(entity.render_order || 0) * 1000 + stackIndex;
            const authoredYaw = Number(entity.rotation_y || 0);
            SPRITESTACK_FACING.setFromAxisAngle(
                SPRITESTACK_Y_AXIS, facingYaw(entity.facing) + authoredYaw,
            );
            SPRITESTACK_AUTHORED_ROTATION.setFromAxisAngle(
                SPRITESTACK_Z_AXIS, -Number(entity.rotation_z || 0),
            );
            mesh.quaternion.copy(SPRITESTACK_FACING)
                .multiply(SPRITESTACK_HORIZONTAL_SLICE)
                .multiply(SPRITESTACK_AUTHORED_ROTATION);
            // Retain Rust-owned animation state on the presentation object;
            // the browser does not derive a second animation clock.
            mesh.userData.animationState = entity.animation_state || 'idle';
            mesh.userData.animationTimeMs = Number(entity.animation_time_ms || 0);
            mesh.userData.animationFrame = entity.animation_frame ?? null;
            mesh.userData.poseSample = rig && animationClip
                ? samplePose(poseCatalog, rig, animationClip, entity.animation_time_ms)
                : null;
            mesh.userData.entityKind = entity.kind;
            if (actorMask) syncMaskMesh(actorMask, mesh, key, texture);
            seen.add(key);
        });
    });
    (frame.entities || []).forEach((entity) => {
        const indicator = entity.indicator;
        if (!indicator || !entity.world_position) return;
        if (indicator.kind === 'facing' && (!entity.asset || !isActorEntity(entity))) return;
        const id = String(entity.id);
        let marker = teamMarkers.get(id);
        const color = Array.isArray(indicator.color)
            ? new THREE.Color(indicator.color[0], indicator.color[1], indicator.color[2])
            : new THREE.Color(indicator.color);
        const state = String(indicator.state || 'committed');
        if (!marker) {
            marker = new THREE.Group();
            const ring = new THREE.Mesh(teamMarkerGeometry, new THREE.MeshBasicMaterial({
                color, transparent: true, opacity: state === 'hover' ? 0.45 : 0.85,
                blending: THREE.AdditiveBlending, depthTest: false, depthWrite: false,
                side: THREE.DoubleSide,
            }));
            ring.rotation.x = -Math.PI / 2;
            marker.add(ring);
            if (indicator.kind === 'facing') {
                const arrow = new THREE.Mesh(facingMarkerGeometry, ring.material.clone());
                arrow.rotation.x = -Math.PI / 2;
                arrow.position.z = -0.46;
                marker.add(arrow);
            }
            scene.add(marker); teamMarkers.set(id, marker);
        } else {
            marker.traverse((child) => {
                if (child.material) {
                    child.material.color.copy(color);
                    child.material.opacity = state === 'hover' ? 0.45 : 0.85;
                }
            });
        }
        marker.position.fromArray(entity.world_position); marker.position.y += 0.16;
        marker.scale.setScalar((Number(entity.scale) || 1) * 1.15);
        const direction = facingYaw(indicator.direction);
        // The arrow mesh points along local -Z; negate the world heading so
        // east/west remain aligned with the authored facing convention.
        marker.rotation.y = direction;
        marker.renderOrder = Number(entity.render_order || 0) * 1000 + 100000;
        seenMarkers.add(id);
    });
    teamMarkers.forEach((marker, id) => { if (!seenMarkers.has(id)) { scene.remove(marker); marker.traverse((child) => child.material?.dispose()); teamMarkers.delete(id); } });
    meshes.forEach((mesh, id) => {
        if (!seen.has(id)) {
            scene.remove(mesh);
            mesh.geometry.dispose();
            mesh.material.dispose();
            meshes.delete(id);
        }
    });
    if (actorMask) actorMask.scene.children.slice().forEach((mesh) => {
        if (mesh.isMesh && !seen.has(mesh.userData.sourceKey)) {
            actorMask.scene.remove(mesh); mesh.material.dispose(); actorMaskMeshes.delete(mesh.userData.sourceKey);
        }
    });
    window.__pystralThreeNativeProfile = {
        entities: new Set([...meshes.keys()].map((key) => key.split(':')[0])).size,
        mapTiles: frame.map?.tiles?.length || window.__pystralThreeNativeProfile?.mapTiles || 0,
        nativeMeshCount: meshes.size,
    };
}

const actorMaskMeshes = new Map();
function syncMaskMesh(mask, source, key, texture) {
    let mesh = actorMaskMeshes.get(key);
    if (!mesh) {
        mesh = new THREE.Mesh(source.geometry, mask.material.clone());
        mesh.userData.sourceKey = key;
        mask.scene.add(mesh);
        actorMaskMeshes.set(key, mesh);
    }
    mesh.position.copy(source.position);
    mesh.quaternion.copy(source.quaternion);
    mesh.scale.copy(source.scale);
    mesh.renderOrder = source.renderOrder;
    mesh.material.uniforms.atlas.value = texture;
}

function isActorEntity(entity) {
    return !['prompt', 'rock', 'world', 'camera', 'projectile'].includes(String(entity.kind).toLowerCase());
}

function addCameraRelativeOffset(position, camera, offset) {
    if (!Array.isArray(offset) || offset.length !== 3) return;
    const elements = camera.matrixWorld.elements;
    position.x += elements[0] * offset[0] + elements[4] * offset[1] - elements[8] * offset[2];
    position.y += elements[1] * offset[0] + elements[5] * offset[1] - elements[9] * offset[2];
    position.z += elements[2] * offset[0] + elements[6] * offset[1] - elements[10] * offset[2];
}

function applyNativeMap(frame, scene) {
    const map = frame.map || window.__pystralThreeStaticMap;
    if (!map) return;
    const tileMeshes = window.__pystralThreeNativeTileMeshes ||
        (window.__pystralThreeNativeTileMeshes = new Map());
    const materials = window.__pystralThreeStaticMaterials || frame.materials || {};
    const orientation = String(map.orientation || '').toLowerCase();
    const pointy = orientation.includes('pointy');
    const [sizeX, sizeZ] = map.hex_size || [1, 1];
    const geometryKey = `${pointy ? 'pointy' : 'flat'}:${sizeX}:${sizeZ}`;
    let geometry = window.__pystralThreeNativeHexGeometry;
    if (!geometry || window.__pystralThreeNativeHexGeometryKey !== geometryKey) {
        geometry?.dispose();
        // CylinderGeometry's radial convention is x=sin(theta), z=cos(theta).
        // Therefore theta=0 puts vertices on the z axis (pointy-top), while
        // theta=PI/6 puts vertices on the x axis (flat-top), matching hexx.
        geometry = new THREE.CylinderGeometry(1, 1, 1, 6, 1, false, pointy ? 0 : Math.PI / 6);
        geometry.scale(sizeX, 1, sizeZ);
        window.__pystralThreeNativeHexGeometry = geometry;
        window.__pystralThreeNativeHexGeometryKey = geometryKey;
    }
    const seen = new Set();
    (map.tiles || []).forEach((tile, index) => {
        const key = `${tile.q}:${tile.r}:${tile.layer}:${index}`;
        const [x, z] = hexCenter(tile.q, tile.r, pointy, sizeX, sizeZ);
        let mesh = tileMeshes.get(key);
        if (!mesh) {
            const definition = materials[tile.material] || {};
            const color = definition.color || [0.5, 0.5, 0.5];
            const tileMaterial = new THREE.MeshStandardMaterial({
                color: new THREE.Color(color[0], color[1], color[2]),
                roughness: definition.roughness ?? 0.8,
                metalness: definition.metalness ?? 0,
                emissive: new THREE.Color(color[0], color[1], color[2]),
                emissiveIntensity: definition.emissive ?? 0,
            });
            mesh = new THREE.Mesh(geometry, tileMaterial);
            mesh.receiveShadow = true;
            scene.add(mesh);
            tileMeshes.set(key, mesh);
        }
        // Zero-height tiles are valid markers/voids. Keep the JS geometry
        // contract identical to Rust's `bottom + height` terrain contract;
        // only a missing or non-finite height receives the compatibility
        // default.
        const parsedHeight = Number(tile.height);
        const height = Number.isFinite(parsedHeight) ? parsedHeight : 1;
        const parsedBottom = Number(tile.bottom);
        const bottom = Number.isFinite(parsedBottom) ? parsedBottom : 0;
        // ColumnMeshBuilder emits columns from y=0 to y=height. The Three.js
        // cylinder is centered, so place its midpoint at bottom + height/2.
        mesh.position.set(x, bottom + height / 2, z);
        mesh.scale.y = height;
        mesh.renderOrder = Number(tile.layer || 0);
        seen.add(key);
    });
    tileMeshes.forEach((mesh, key) => {
        if (!seen.has(key)) {
            scene.remove(mesh);
            mesh.material.dispose();
            tileMeshes.delete(key);
        }
    });
    updateCompass(map, pointy, sizeX, sizeZ, scene, window.__pystralThreeNativeCamera);
    window.__pystralThreeNativeProfile = {
        ...(window.__pystralThreeNativeProfile || {}),
        mapTiles: tileMeshes.size,
    };
}

function updateCompass(map, pointy, sizeX, sizeZ, scene, camera) {
    const lowest = (map.tiles || []).reduce((best, tile) =>
        !best || Number(tile.bottom || 0) < Number(best.bottom || 0) ? tile : best, null);
    if (!lowest) return;
    const bottom = Number.isFinite(Number(lowest.bottom)) ? Number(lowest.bottom) : 0;
    const height = Number.isFinite(Number(lowest.height)) ? Number(lowest.height) : 1;
    const compassY = bottom + height + 0.02;
    const [anchorX, anchorZ] = hexCenter(lowest.q, lowest.r, pointy, sizeX, sizeZ);
    const anchor = [anchorX, compassY, anchorZ];
    let compass = window.__pystralThreeCompass;
    if (!compass) {
        compass = new THREE.Group();
        const ring = new THREE.Mesh(new THREE.RingGeometry(0.55, 0.60, 24), new THREE.MeshBasicMaterial({
            color: 0xf5d76e, transparent: true, opacity: 0.8, depthTest: true, depthWrite: false,
        }));
        ring.rotation.x = -Math.PI / 2;
        compass.add(ring);
        ['N', 'NE', 'SE', 'S', 'SW', 'NW'].forEach((label, index) => {
            const texture = new THREE.CanvasTexture(compassLabelCanvas(label));
            texture.minFilter = THREE.NearestFilter;
            const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: true, depthWrite: false }));
            const angle = index * Math.PI / 3 + HEX_DIRECTION_OFFSET;
            sprite.position.set(Math.cos(angle) * 0.78, 0.02, Math.sin(angle) * 0.78);
            sprite.scale.set(0.28, 0.14, 1);
            compass.add(sprite);
        });
        scene.add(compass);
        window.__pystralThreeCompass = compass;
    }
    compass.position.set(anchor[0], anchor[1], anchor[2]);
    const elements = camera?.matrixWorld.elements;
    if (elements) compass.rotation.y = Math.atan2(elements[8], elements[10]);
    compass.renderOrder = 200000;
}

function compassLabelCanvas(label) {
    const canvas = document.createElement('canvas');
    canvas.width = 96; canvas.height = 48;
    const context = canvas.getContext('2d');
    context.font = 'bold 30px sans-serif';
    context.textAlign = 'center'; context.textBaseline = 'middle';
    context.fillStyle = '#fff4b0'; context.fillText(label, 48, 24);
    return canvas;
}

function createFpsBucket() {
    return {
        frames: 0,
        elapsedMs: 0,
        renderMs: 0,
        lastFrameAt: 0,
        maxFrameIntervalMs: 0,
        droppedFrames: 0,
        frameIntervalsMs: [],
    };
}

function recordFpsSample(bucket, frameStart, renderMs, targetFps) {
    bucket.frames += 1;
    if (bucket.lastFrameAt > 0) {
        const intervalMs = frameStart - bucket.lastFrameAt;
        const expectedMs = 1000 / targetFps;
        bucket.elapsedMs += intervalMs;
        bucket.maxFrameIntervalMs = Math.max(bucket.maxFrameIntervalMs, intervalMs);
        bucket.frameIntervalsMs.push(intervalMs);
        if (bucket.frameIntervalsMs.length > 120) bucket.frameIntervalsMs.shift();
        if (intervalMs > expectedMs * 1.5) {
            bucket.droppedFrames += Math.max(1, Math.round(intervalMs / expectedMs) - 1);
        }
    }
    bucket.renderMs += renderMs;
    bucket.lastFrameAt = frameStart;
}

function phaseForStatus(status) {
    if (status === 'AwaitingPlayerDecision' || status === 'Completed' || status === 'Idle') {
        return 'acquiescent';
    }
    if (status === 'Simulating' || status === 'WaitingForAnimationAck'
        || status === 'MctsThinking' || status === 'MctsExpanding') {
        return 'simulation';
    }
    return null;
}

function cameraSignatureFor(frame) {
    const cameras = frame?.cameras;
    if (!Array.isArray(cameras) || cameras.length === 0) return null;
    return cameras.map((entry) => `${entry.id}:${Number(entry.angle).toFixed(5)}`).join('|');
}

function snapshotFps(profile) {
    const result = {};
    for (const [phase, modes] of Object.entries(profile.fps)) {
        result[phase] = {};
        for (const [motion, bucket] of Object.entries(modes)) {
            result[phase][motion] = {
                frames: bucket.frames,
                elapsedMs: bucket.elapsedMs,
                fps: bucket.elapsedMs > 0 ? bucket.frames * 1000 / bucket.elapsedMs : 0,
                averageRenderMs: bucket.frames > 0 ? bucket.renderMs / bucket.frames : 0,
                p95FrameIntervalMs: percentile(bucket.frameIntervalsMs, 0.95),
                maxFrameIntervalMs: bucket.maxFrameIntervalMs,
                droppedFrames: bucket.droppedFrames,
                lastFrameAt: bucket.lastFrameAt,
            };
        }
    }
    return result;
}

function percentile(values, fraction) {
    if (!values.length) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}
