// Readable semantic colors (authored in OKLCH, represented as sRGB for Three).
const PALETTE = Object.freeze({
    // White is intentional: entities without a semantic team must retain the
    // authored sprite colors instead of being gray-tinted by multiplication.
    neutral: '#FFFFFF', player: '#8CCCF2', ally: '#82D8C9',
    enemy: '#F5A08C', elite: '#D7A6EA',
});

export function teamRole(teamId) {
    const id = Number(teamId);
    if (!Number.isFinite(id) || id <= 0) return 'neutral';
    return id === 1 ? 'player' : id === 2 ? 'enemy' : 'ally';
}

export function semanticColor(teamId) { return PALETTE[teamRole(teamId)]; }
