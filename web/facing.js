// Canonical presentation heading for tactical Facing values.  Both the
// authoritative direction marker and every GLB actor use this convention.
export const HEX_DIRECTION_OFFSET = Math.PI / 6;

const FACING_ANGLES = Object.freeze({
    north: HEX_DIRECTION_OFFSET,
    northeast: Math.PI / 3 + HEX_DIRECTION_OFFSET,
    southeast: (Math.PI * 2) / 3 + HEX_DIRECTION_OFFSET,
    south: Math.PI + HEX_DIRECTION_OFFSET,
    southwest: (Math.PI * 4) / 3 + HEX_DIRECTION_OFFSET,
    northwest: (Math.PI * 5) / 3 + HEX_DIRECTION_OFFSET,
});

// Direction markers point down their local -Z axis.  This is consequently
// the group yaw that makes their visible arrow point at a logical Facing.
export function facingYaw(facing) {
    const angle = FACING_ANGLES[String(facing || '').toLowerCase()];
    return angle === undefined ? 0 : -angle;
}

// GLB assets may have an authored local forward that differs from the marker
// mesh.  Keep that calibration declarative and additive to logical facing.
export function actorYaw(manifest, asset, facing, authoredYaw = 0) {
    const entry = manifest?.models?.[asset] || {};
    const modelForwardYaw = Number(entry.forward_yaw ?? manifest?.model_forward_yaw ?? 0);
    return facingYaw(facing) + (Number.isFinite(modelForwardYaw) ? modelForwardYaw : 0)
        + (Number(authoredYaw) || 0);
}
