/* tslint:disable */
/* eslint-disable */
/**
 * The `ReadableStreamType` enum.
 *
 * *This API requires the following crate features to be activated: `ReadableStreamType`*
 */

export type ReadableStreamType = "bytes";

export class IntoUnderlyingByteSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableByteStreamController): Promise<any>;
    start(controller: ReadableByteStreamController): void;
    readonly autoAllocateChunkSize: number;
    readonly type: ReadableStreamType;
}

export class IntoUnderlyingSink {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    abort(reason: any): Promise<any>;
    close(): Promise<any>;
    write(chunk: any): Promise<any>;
}

export class IntoUnderlyingSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableStreamDefaultController): Promise<any>;
}

export class JSOwner {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
}

export function __ev_start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __ev_start: () => void;
    readonly __wbg_jsowner_free: (a: number, b: number) => void;
    readonly __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
    readonly __wbg_intounderlyingsource_free: (a: number, b: number) => void;
    readonly intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
    readonly intounderlyingbytesource_cancel: (a: number) => void;
    readonly intounderlyingbytesource_pull: (a: number, b: any) => any;
    readonly intounderlyingbytesource_start: (a: number, b: any) => void;
    readonly intounderlyingbytesource_type: (a: number) => number;
    readonly intounderlyingsource_cancel: (a: number) => void;
    readonly intounderlyingsource_pull: (a: number, b: any) => any;
    readonly __wbg_intounderlyingsink_free: (a: number, b: number) => void;
    readonly intounderlyingsink_abort: (a: number, b: any) => any;
    readonly intounderlyingsink_close: (a: number) => any;
    readonly intounderlyingsink_write: (a: number, b: any) => any;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___alloc_b4e02328d93dd30e___vec__Vec_u32___js_sys_dd8c905130477655___Uint8Array__core_74df32bb1ec6ca0a___option__Option_alloc_b4e02328d93dd30e___vec__Vec_alloc_b4e02328d93dd30e___string__String____core_74df32bb1ec6ca0a___option__Option_alloc_b4e02328d93dd30e___vec__Vec_alloc_b4e02328d93dd30e___string__String________true_: (a: number, b: number, c: number, d: number, e: any, f: number, g: number, h: number, i: number) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___wasm_bindgen_c01c63822dad0eba___JsValue__core_74df32bb1ec6ca0a___result__Result_____wasm_bindgen_c01c63822dad0eba___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___js_sys_dd8c905130477655___Function_fn_wasm_bindgen_c01c63822dad0eba___JsValue_____wasm_bindgen_c01c63822dad0eba___sys__Undefined___js_sys_dd8c905130477655___Function_fn_wasm_bindgen_c01c63822dad0eba___JsValue_____wasm_bindgen_c01c63822dad0eba___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___wasm_bindgen_c01c63822dad0eba___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___web_sys_9e4fbfbd29414117___features__gen_Element__Element______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke___web_sys_9e4fbfbd29414117___features__gen_Event__Event______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures________invoke___web_sys_9e4fbfbd29414117___features__gen_Event__Event______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_c01c63822dad0eba___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
