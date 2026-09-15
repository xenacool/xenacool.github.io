const DB_NAME = 'pystral-gate-workspaces';
const DB_VERSION = 1;
const WORKSPACES = 'workspaces';
const SETTINGS = 'settings';
export const BUILTIN_PG_RPG = 'builtin:pg_rpg';
const ENTRYPOINT = 'scripts/pg_rpg.rhai';

function openDatabase() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);
        request.onupgradeneeded = () => {
            const db = request.result;
            db.createObjectStore(WORKSPACES, { keyPath: 'id' });
            db.createObjectStore(SETTINGS, { keyPath: 'key' });
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

async function transaction(store, mode, operation) {
    const db = await openDatabase();
    try {
        return await new Promise((resolve, reject) => {
            const request = operation(db.transaction(store, mode).objectStore(store));
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    } finally { db.close(); }
}

function validPath(path) {
    return typeof path === 'string' && path.startsWith('scripts/')
        && !path.includes('..') && !path.includes('//') && path.length < 256;
}

function assertWorkspace(record) {
    if (!record || record.schemaVersion !== 1 || typeof record.id !== 'string'
        || !record.id || typeof record.name !== 'string' || !record.name.trim()
        || record.templateId !== BUILTIN_PG_RPG || record.entrypoint !== ENTRYPOINT
        || typeof record.templateFingerprint !== 'string' || !record.templateFingerprint
        || !record.overrides || typeof record.overrides !== 'object'
        || Array.isArray(record.overrides)) throw new Error('Invalid saved workspace record.');
    for (const [path, contents] of Object.entries(record.overrides)) {
        if (!validPath(path) || typeof contents !== 'string') throw new Error(`Invalid workspace override: ${path}`);
    }
    return record;
}

async function fingerprint(files) {
    const source = [...files].sort((a, b) => a.path.localeCompare(b.path))
        .map(({ path, contents }) => `${path}\0${contents}\0`).join('');
    const bytes = new TextEncoder().encode(source);
    const digest = await crypto.subtle.digest('SHA-256', bytes);
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

export async function loadBuiltinTemplate() {
    const response = await fetch('./web/scripts/manifest.json', { cache: 'no-store' });
    if (!response.ok) throw new Error(`Cannot load script manifest: HTTP ${response.status}`);
    const manifest = await response.json();
    if (!Array.isArray(manifest.files)) throw new Error('Invalid script manifest.');
    const files = await Promise.all(manifest.files.map(async (path) => {
        if (typeof path !== 'string' || !path || path.includes('..')) throw new Error(`Invalid manifest path: ${path}`);
        const file = await fetch(`./web/scripts/${path}`, { cache: 'no-store' });
        if (!file.ok) throw new Error(`Cannot load script ${path}: HTTP ${file.status}`);
        return { path: `scripts/${path}`, contents: await file.text() };
    }));
    return { id: BUILTIN_PG_RPG, name: 'Built-in pg_rpg', entrypoint: ENTRYPOINT,
        files, fingerprint: await fingerprint(files) };
}

export async function listWorkspaces() {
    const records = await transaction(WORKSPACES, 'readonly', (store) => store.getAll());
    return records.map(assertWorkspace).sort((a, b) => a.name.localeCompare(b.name));
}

export async function getWorkspace(id) {
    if (id === BUILTIN_PG_RPG) return null;
    return transaction(WORKSPACES, 'readonly', (store) => store.get(id)).then((record) => {
        if (!record) throw new Error(`Saved workspace does not exist: ${id}`);
        return assertWorkspace(record);
    });
}

export async function createWorkspace(name, template) {
    const now = new Date().toISOString();
    const record = { schemaVersion: 1, id: crypto.randomUUID(), name: name.trim(), templateId: template.id,
        templateFingerprint: template.fingerprint, entrypoint: template.entrypoint, overrides: {}, createdAt: now, updatedAt: now };
    assertWorkspace(record);
    await transaction(WORKSPACES, 'readwrite', (store) => store.put(record));
    return record;
}

export async function saveWorkspace(record) {
    const next = { ...record, name: record.name.trim(), updatedAt: new Date().toISOString() };
    assertWorkspace(next);
    await transaction(WORKSPACES, 'readwrite', (store) => store.put(next));
    return next;
}

export async function deleteWorkspace(id) {
    if (id === BUILTIN_PG_RPG) throw new Error('The built-in workspace cannot be deleted.');
    await transaction(WORKSPACES, 'readwrite', (store) => store.delete(id));
    if (await selectedWorkspaceId() === id) await selectWorkspace(null);
}

export async function selectedWorkspaceId() {
    const setting = await transaction(SETTINGS, 'readonly', (store) => store.get('selected-workspace'));
    return setting?.value ?? null;
}

export async function selectWorkspace(id) {
    if (id !== null && id !== BUILTIN_PG_RPG) await getWorkspace(id);
    await transaction(SETTINGS, 'readwrite', (store) => store.put({ key: 'selected-workspace', value: id }));
}

export async function materializeWorkspace(id, template) {
    if (!template) throw new Error('A validated template is required to materialize a workspace.');
    if (id === BUILTIN_PG_RPG) return { entrypoint: template.entrypoint, files: template.files };
    const saved = await getWorkspace(id);
    if (saved.templateFingerprint !== template.fingerprint) {
        throw new Error('Saved workspace template changed. Open it in the editor and explicitly migrate it before playing.');
    }
    const known = new Map(template.files.map((file) => [file.path, file.contents]));
    for (const [path, contents] of Object.entries(saved.overrides)) {
        if (!known.has(path)) throw new Error(`Saved workspace overrides a removed template file: ${path}`);
        known.set(path, contents);
    }
    return { entrypoint: saved.entrypoint, files: [...known].map(([path, contents]) => ({ path, contents })) };
}
