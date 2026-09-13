// Extracted from Three.js r178 examples/jsm/utils/SkeletonUtils.js (MIT).
// Keep this small vendored subset beside the matching r178 runtime: GLB actor
// instances only need skeleton-safe cloning, not animation retargeting.

export function clone(source) {
    const sourceLookup = new Map();
    const cloneLookup = new Map();
    const cloned = source.clone(true);

    parallelTraverse(source, cloned, (sourceNode, clonedNode) => {
        sourceLookup.set(clonedNode, sourceNode);
        cloneLookup.set(sourceNode, clonedNode);
    });

    cloned.traverse((node) => {
        if (!node.isSkinnedMesh) return;
        const sourceMesh = sourceLookup.get(node);
        const sourceBones = sourceMesh.skeleton.bones;
        node.skeleton = sourceMesh.skeleton.clone();
        node.bindMatrix.copy(sourceMesh.bindMatrix);
        node.skeleton.bones = sourceBones.map((bone) => cloneLookup.get(bone));
        node.bind(node.skeleton, node.bindMatrix);
    });
    return cloned;
}

function parallelTraverse(source, cloned, callback) {
    callback(source, cloned);
    for (let index = 0; index < source.children.length; index += 1) {
        parallelTraverse(source.children[index], cloned.children[index], callback);
    }
}
