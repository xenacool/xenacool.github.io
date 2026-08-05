import init, { init_worker } from './web/pystral_gate.js';

async function start() {
    try {
        await init('/web/pystral_gate_bg.wasm');
        init_worker();
    } catch (err) {
        console.error('Worker initialization failed:', err);
    }
}

start();
