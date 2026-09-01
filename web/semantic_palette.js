// Readable semantic colors (authored in OKLCH, represented as sRGB for Three).
const PALETTE = Object.freeze({
    neutral: '#F2F4F7', player: '#4EA8DE', ally: '#2A9D8F',
    enemy: '#E76F51', elite: '#B565D9',
});

export function teamRole(teamId) {
    const id = Number(teamId);
    if (!Number.isFinite(id) || id <= 0) return 'neutral';
    return id === 1 ? 'player' : id === 2 ? 'enemy' : 'ally';
}

export function semanticColor(teamId) { return PALETTE[teamRole(teamId)]; }
