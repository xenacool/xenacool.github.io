/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly __wasm_bindgen_func_elem_106033: (a: number, b: number, c: any, d: any) => void;
    readonly __wasm_bindgen_func_elem_95896: (a: number, b: number, c: number, d: any) => void;
    readonly __wasm_bindgen_func_elem_102132: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_10: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_5: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_6: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_7: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_8: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105918_9: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_105921: (a: number, b: number, c: any) => void;
    readonly __wasm_bindgen_func_elem_106005: (a: number, b: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export4: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_export5: (a: number) => void;
    readonly __wbindgen_export6: (a: number, b: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export7: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
