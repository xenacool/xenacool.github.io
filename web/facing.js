// Yaws for a local -Z forward axis on the pointy-top tactical layout. These
// are derived from Facing::from_delta, not a separately rotated compass.
export const HEX_DIRECTION_OFFSET = Math.PI / 6;
const FACING_YAWS = Object.freeze({
    north: HEX_DIRECTION_OFFSET,
    northeast: -HEX_DIRECTION_OFFSET,
    southeast: -Math.PI / 2,
    south: -Math.PI * 5 / 6,
    southwest: -Math.PI * 7 / 6,
    northwest: Math.PI / 2,
});

// Direction markers point down their local -Z axis.  This is consequently
// the group yaw that makes their visible arrow point at a logical Facing.
export function facingYaw(facing) {
    const yaw = FACING_YAWS[String(facing || '').toLowerCase()];
    return yaw === undefined ? 0 : yaw;
}

// GLB assets may have an authored local forward that differs from the marker
// mesh.  Keep that calibration declarative and additive to logical facing.
export function actorModelYaw(manifest, asset, authoredYaw = 0) {
    const entry = manifest?.models?.[asset] || {};
    const modelForwardYaw = Number(entry.forward_yaw ?? manifest?.model_forward_yaw ?? 0);
    return (Number.isFinite(modelForwardYaw) ? modelForwardYaw : 0) + (Number(authoredYaw) || 0);
}

export function actorYaw(manifest, asset, facing, authoredYaw = 0) {
    return facingYaw(facing) + actorModelYaw(manifest, asset, authoredYaw);
}
