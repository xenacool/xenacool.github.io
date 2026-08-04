import init, { run_rhai_case } from './web/pystral_gate.js';

let ready;
self.onmessage = async ({ data }) => {
    try {
        ready ||= init({ module_or_path: 'web/pystral_gate_bg.wasm' });
        await ready;
        const result = run_rhai_case(data.workspace, data.caseName, BigInt(data.seed));
        self.postMessage({ id: data.id, result });
    } catch (error) {
        self.postMessage({ id: data.id, error: String(error) });
    }
};
