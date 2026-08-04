import * as fs from 'fs';
import * as path from 'path';

type Catalog = {
  rigs: Record<string, { clips: Array<{ name: string; duration_ms: number }> }>;
};
type Mapping = { state: string; semantic: string; candidates: Record<string, string[]> };

const ROOT = process.cwd();
const CATALOG_FILE = path.join(ROOT, 'web/animation_catalog.json');
const SOURCE_ROOTS = [path.join(ROOT, 'crates'), path.join(ROOT, 'web/scripts'), path.join(ROOT, 'tests')];

// Runtime names are deliberately separate from source clip names. The first
// candidate is the conservative default for a future bake manifest.
const SEMANTIC_CANDIDATES: Record<string, string[]> = {
  idle: ['Idle_A', 'Idle_B'],
  walk: ['Walking_A', 'Walking_B', 'Walking_C'],
  walking: ['Walking_A', 'Walking_B', 'Walking_C'],
  run: ['Running_A', 'Running_B'],
  running: ['Running_A', 'Running_B'],
  hit: ['Hit_A', 'Hit_B'],
  death: ['Death_A', 'Death_B'],
  attack: [
    'Melee_1H_Attack_Chop',
    'Melee_1H_Attack_Slice_Diagonal',
    'Melee_1H_Attack_Stab',
    'Melee_2H_Attack',
    'Melee_2H_Attack_Chop',
    'Melee_Block_Attack',
    'Melee_Unarmed_Attack_Punch_A',
  ],
};

function sourceFiles(directory: string): string[] {
  if (!fs.existsSync(directory)) return [];
  const files: string[] = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...sourceFiles(fullPath));
    else if (/\.(rs|rhai|js|ts)$/.test(entry.name)) files.push(fullPath);
  }
  return files;
}

function discoverStates(): string[] {
  const states = new Set(['idle']);
  const patterns = [
    /animation_state\s*:\s*["']([A-Za-z0-9_-]+)["']/g,
    /(?:set_animation_state|new_animation_state|add_state)\s*\([^,)]*,?\s*["']([A-Za-z0-9_-]+)["']/g,
  ];
  for (const root of SOURCE_ROOTS) {
    for (const file of sourceFiles(root)) {
      const source = fs.readFileSync(file, 'utf8');
      for (const pattern of patterns) {
        for (const match of source.matchAll(pattern)) states.add(match[1].toLowerCase());
      }
    }
  }
  return [...states].sort();
}

function semanticFor(state: string): string {
  const normalized = state.toLowerCase().replace(/[-_ ]/g, '');
  if (normalized.includes('death') || normalized.includes('dead')) return 'death';
  if (normalized.includes('hit') || normalized.includes('hurt')) return 'hit';
  if (normalized.includes('attack') || normalized.includes('cast')) return 'attack';
  if (normalized.includes('walk')) return 'walking';
  if (normalized.includes('run')) return 'running';
  return normalized === 'idle' ? 'idle' : state.toLowerCase();
}

function main() {
  const catalog = JSON.parse(fs.readFileSync(CATALOG_FILE, 'utf8')) as Catalog;
  const states = discoverStates();
  const mappings: Mapping[] = states.map((state) => {
    const semantic = semanticFor(state);
    const requested = SEMANTIC_CANDIDATES[semantic] || [];
    const candidates: Record<string, string[]> = {};
    for (const [rig, data] of Object.entries(catalog.rigs)) {
      const available = new Set(data.clips.map((clip) => clip.name));
      candidates[rig] = requested.filter((name) => available.has(name));
    }
    return { state, semantic, candidates };
  });
  const missing = mappings.flatMap((mapping) => Object.entries(mapping.candidates)
    .filter(([, candidates]) => candidates.length === 0)
    .map(([rig]) => `${mapping.state} (${rig})`));
  const selectionRequired = mappings.flatMap((mapping) => Object.entries(mapping.candidates)
    .filter(([, candidates]) => candidates.length > 1)
    .map(([rig, candidates]) => `${mapping.state} (${rig}): ${candidates.length} candidates`));
  console.log(JSON.stringify({
    version: 1,
    source: 'runtime state literals and defaults',
    states,
    mappings,
    missing,
    selection_required: selectionRequired,
    bake_ready: missing.length === 0 && selectionRequired.length === 0,
  }, null, 2));
}

main();
