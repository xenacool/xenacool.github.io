// Deterministic planar two-bone IK helper for presentation overlays.
// Inputs/outputs are plain numbers so it can be used without Three.js.
export function solveTwoBoneIK(root, target, upperLength, lowerLength) {
    const dx = Number(target[0]) - Number(root[0]);
    const dy = Number(target[1]) - Number(root[1]);
    const distance = Math.hypot(dx, dy);
    const minReach = Math.abs(upperLength - lowerLength);
    const maxReach = upperLength + lowerLength;
    const reach = Math.max(minReach, Math.min(maxReach, distance));
    const baseAngle = Math.atan2(dy, dx);
    const elbowCos = (upperLength ** 2 + reach ** 2 - lowerLength ** 2)
        / Math.max(1e-9, 2 * upperLength * reach);
    const elbowAngle = Math.acos(Math.max(-1, Math.min(1, elbowCos)));
    const upperAngle = baseAngle - elbowAngle;
    const elbow = [
        Number(root[0]) + upperLength * Math.cos(upperAngle),
        Number(root[1]) + upperLength * Math.sin(upperAngle),
    ];
    const end = [
        Number(root[0]) + reach * Math.cos(baseAngle),
        Number(root[1]) + reach * Math.sin(baseAngle),
    ];
    return { upperAngle, lowerAngle: baseAngle + elbowAngle, elbow, end, clamped: distance !== reach };
}

export function blendAngle(from, to, amount) {
    const t = Math.max(0, Math.min(1, Number(amount) || 0));
    let delta = (to - from + Math.PI) % (Math.PI * 2) - Math.PI;
    return from + delta * t;
}

export function blendTwoBonePose(previous, solved, amount) {
    if (!previous) return { upperAngle: solved.upperAngle, lowerAngle: solved.lowerAngle };
    return {
        upperAngle: blendAngle(previous.upperAngle, solved.upperAngle, amount),
        lowerAngle: blendAngle(previous.lowerAngle, solved.lowerAngle, amount),
    };
}
