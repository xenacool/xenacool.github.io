import init, { init_simulation_worker } from './web/pystral_gate.js';

async function start() {
    try {
        await init({ module_or_path: 'web/pystral_gate_bg.wasm' });
        init_simulation_worker();
    } catch (err) {
        console.error('Simulation worker initialization failed:', err);
    }
}

start();
