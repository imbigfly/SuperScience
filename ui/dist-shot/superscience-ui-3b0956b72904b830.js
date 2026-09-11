import { add_workspace_file_to_motif, attach_cropped_region, close_mcp_app, crop_region_to_upload, download_app_update, import_motif_dna_file, invoke, invoke_strict, invoke_timeout, listen, listen_current_window, media_thumbnail_url, media_url, mount_mcp_app, mount_preview, park_mcp_app, preview_selection, read_feedback_input_files, render_share_png, request_motif_selection, set_window_title, snapshot_share_theme, upload_input_files, window_control } from './snippets/superscience-ui-50d348c2a8229c6d/src/api.js';
import { copyImage } from './snippets/superscience-ui-50d348c2a8229c6d/src/context_menu.js';
import { highlight_by_id, highlight_set_code } from './snippets/superscience-ui-50d348c2a8229c6d/src/highlight.js';
import { reveal_saved_mark, set_saved_marks } from './snippets/superscience-ui-50d348c2a8229c6d/src/marks.js';
import { sync_pet } from './snippets/superscience-ui-50d348c2a8229c6d/src/pet.js';
import { attach_chat_scroll, force_chat_scroll_bottom, jump_chat_scroll, notify_chat_scroll, preserve_chat_scroll_on_prepend, switch_chat_scroll } from './snippets/superscience-ui-50d348c2a8229c6d/src/scroll.js';
import { mount_terminal, set_terminal_active, unmount_terminal } from './snippets/superscience-ui-50d348c2a8229c6d/src/terminal.js';
import * as __wbg_star0 from './snippets/superscience-ui-50d348c2a8229c6d/src/api.js';
import * as __wbg_star1 from './snippets/superscience-ui-50d348c2a8229c6d/src/context_menu.js';
import * as __wbg_star2 from './snippets/superscience-ui-50d348c2a8229c6d/src/marks.js';
import * as __wbg_star3 from './snippets/superscience-ui-50d348c2a8229c6d/src/scroll.js';

let wasm;

let WASM_VECTOR_LEN = 0;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getFromExternrefTable0(idx) { return wasm.__wbindgen_export_2.get(idx); }

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getFromExternrefTable0(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_2.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b)
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_6.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_6.get(state.dtor)(state.a, state.b);
                state.a = 0;
                CLOSURE_DTORS.unregister(state);
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}
function __wbg_adapter_58(arg0, arg1, arg2) {
    wasm.closure743_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_61(arg0, arg1, arg2) {
    wasm.closure745_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_64(arg0, arg1, arg2) {
    wasm.closure747_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_67(arg0, arg1, arg2) {
    wasm.closure749_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_70(arg0, arg1, arg2) {
    wasm.closure751_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_73(arg0, arg1, arg2) {
    wasm.closure753_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_76(arg0, arg1, arg2) {
    wasm.closure755_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_79(arg0, arg1, arg2) {
    wasm.closure757_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_82(arg0, arg1, arg2) {
    wasm.closure759_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_85(arg0, arg1, arg2) {
    wasm.closure761_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_88(arg0, arg1, arg2) {
    wasm.closure13892_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_91(arg0, arg1) {
    wasm._dyn_core_9b3796e30d99ddb7___ops__function__FnMut_____Output______as_wasm_bindgen_f84653c2be6ef3c0___closure__WasmClosure___describe__invoke______(arg0, arg1);
}

function __wbg_adapter_94(arg0, arg1, arg2) {
    wasm.closure13960_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_653(arg0, arg1, arg2, arg3) {
    wasm.closure14012_externref_shim(arg0, arg1, arg2, arg3);
}

const __wbindgen_enum_ReadableStreamType = ["bytes"];

const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr >>> 0, 1));

export class IntoUnderlyingByteSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, controller);
        return ret;
    }
    /**
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, controller);
    }
    /**
     * @returns {ReadableStreamType}
     */
    get type() {
        const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
        return __wbindgen_enum_ReadableStreamType[ret];
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
}

const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr >>> 0, 1));

export class IntoUnderlyingSink {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, reason);
        return ret;
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return ret;
    }
    /**
     * @param {any} chunk
     * @returns {Promise<any>}
     */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, chunk);
        return ret;
    }
}

const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr >>> 0, 1));

export class IntoUnderlyingSource {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, controller);
        return ret;
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_String_8f0eb39a4a4c2f66 = function(arg0, arg1) {
        const ret = String(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_addEventListener_85f4089e98a15a56 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.addEventListener(v0, arg3, arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_c2807adc9b10b18a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.addEventListener(v0, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_d1c5eabbfa86ea3b = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.addEventListener(v0, arg3);
    }, arguments) };
    imports.wbg.__wbg_add_1a5abd567c3ea669 = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.add(v0);
    }, arguments) };
    imports.wbg.__wbg_addworkspacefiletomotif_eb310403ab771505 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        const ret = add_workspace_file_to_motif(v0, v1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_altKey_785d01ef6c145e25 = function(arg0) {
        const ret = arg0.altKey;
        return ret;
    };
    imports.wbg.__wbg_animationName_c8488e789f406f7a = function(arg0, arg1) {
        const ret = arg1.animationName;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_appendChild_16091fbec17c52ff = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.appendChild(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_append_1994b6ec080061ec = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.append(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_append_294b332700687838 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.append(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_append_d0537135f413cc9b = function() { return handleError(function (arg0, arg1) {
        arg0.append(arg1);
    }, arguments) };
    imports.wbg.__wbg_attachchatscroll_6c7cd7f0eff56433 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        attach_chat_scroll(v0, v1);
    };
    imports.wbg.__wbg_attachcroppedregion_29303e5a10e77105 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        attach_cropped_region(v0, arg2 !== 0);
    };
    imports.wbg.__wbg_before_e8f929f5fa9ef0ea = function() { return handleError(function (arg0, arg1) {
        arg0.before(arg1);
    }, arguments) };
    imports.wbg.__wbg_body_be7e151b4bdf8dc5 = function(arg0) {
        const ret = arg0.body;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_bottom_180627b4635da67b = function(arg0) {
        const ret = arg0.bottom;
        return ret;
    };
    imports.wbg.__wbg_buffer_742a8f358eb2a863 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_buffer_de79ba23a528d5aa = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_button_ae41c4159d1532ef = function(arg0) {
        const ret = arg0.button;
        return ret;
    };
    imports.wbg.__wbg_buttons_c55f927bf379cb1b = function(arg0) {
        const ret = arg0.buttons;
        return ret;
    };
    imports.wbg.__wbg_byobRequest_d44c380cffc7b6af = function(arg0) {
        const ret = arg0.byobRequest;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_byteLength_4aaa04787b5c2eea = function(arg0) {
        const ret = arg0.byteLength;
        return ret;
    };
    imports.wbg.__wbg_byteOffset_33240b27cf27542d = function(arg0) {
        const ret = arg0.byteOffset;
        return ret;
    };
    imports.wbg.__wbg_call_936d6df407a2ad7c = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_be43a5570f4d90c0 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_cancelBubble_b54b3e0173f8a033 = function(arg0) {
        const ret = arg0.cancelBubble;
        return ret;
    };
    imports.wbg.__wbg_cells_4f244aa9a4d8003b = function(arg0) {
        const ret = arg0.cells;
        return ret;
    };
    imports.wbg.__wbg_checked_55b95e46ba85d978 = function(arg0) {
        const ret = arg0.checked;
        return ret;
    };
    imports.wbg.__wbg_childNodes_3230fa89d60ec66c = function(arg0) {
        const ret = arg0.childNodes;
        return ret;
    };
    imports.wbg.__wbg_classList_9a99e6c82ff061e3 = function(arg0) {
        const ret = arg0.classList;
        return ret;
    };
    imports.wbg.__wbg_clearInterval_c13765c4e920f25b = function(arg0, arg1) {
        arg0.clearInterval(arg1);
    };
    imports.wbg.__wbg_clearTimeout_e6438e33f4fd7adf = function(arg0, arg1) {
        arg0.clearTimeout(arg1);
    };
    imports.wbg.__wbg_click_fa2097e96e699f58 = function(arg0) {
        arg0.click();
    };
    imports.wbg.__wbg_clientHeight_093cdd9592e8c30f = function(arg0) {
        const ret = arg0.clientHeight;
        return ret;
    };
    imports.wbg.__wbg_clientWidth_c3c6a5e5ea48af0b = function(arg0) {
        const ret = arg0.clientWidth;
        return ret;
    };
    imports.wbg.__wbg_clientX_580482b0ad2f4ba8 = function(arg0) {
        const ret = arg0.clientX;
        return ret;
    };
    imports.wbg.__wbg_clientY_d60bb1dbc3af487f = function(arg0) {
        const ret = arg0.clientY;
        return ret;
    };
    imports.wbg.__wbg_clipboard_d83fd708af43d6e2 = function(arg0) {
        const ret = arg0.clipboard;
        return ret;
    };
    imports.wbg.__wbg_cloneNode_148fa8296fe77d69 = function() { return handleError(function (arg0) {
        const ret = arg0.cloneNode();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_close_a70ff32a42ac9ccd = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_close_e3d9f2793489e88b = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_closemcpapp_54d773dcb3a4f8a7 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        close_mcp_app(v0);
    };
    imports.wbg.__wbg_closest_41423f350a5351fe = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.closest(v0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_composedPath_540a7101db52ac25 = function(arg0) {
        const ret = arg0.composedPath();
        return ret;
    };
    imports.wbg.__wbg_contains_3019e86f3ab83ad5 = function(arg0, arg1) {
        const ret = arg0.contains(arg1);
        return ret;
    };
    imports.wbg.__wbg_contains_ebeba9e7ea5268f3 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.contains(v0);
        return ret;
    };
    imports.wbg.__wbg_copyImage_2065281ec4d5e256 = function() { return handleError(function (arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = copyImage(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createComment_d987c22e8d4d0c68 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.createComment(v0);
        return ret;
    };
    imports.wbg.__wbg_createDocumentFragment_9900de791f9a4787 = function(arg0) {
        const ret = arg0.createDocumentFragment();
        return ret;
    };
    imports.wbg.__wbg_createElementNS_48546d9112300b49 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        const ret = arg0.createElementNS(v0, v1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createElement_51ffce71068444e7 = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.createElement(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createTextNode_3ee515eaa2c281b6 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.createTextNode(v0);
        return ret;
    };
    imports.wbg.__wbg_cropregiontoupload_08ccd18d2de0d579 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = crop_region_to_upload(v0, arg2, arg3, arg4, arg5);
        return ret;
    };
    imports.wbg.__wbg_ctrlKey_44e4c750431e84f8 = function(arg0) {
        const ret = arg0.ctrlKey;
        return ret;
    };
    imports.wbg.__wbg_currentTarget_8014c8d0ca31b0f6 = function(arg0) {
        const ret = arg0.currentTarget;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_dataTransfer_1d0aedb15e99a61c = function(arg0) {
        const ret = arg0.dataTransfer;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_defaultPrevented_701cf4646cf0d29e = function(arg0) {
        const ret = arg0.defaultPrevented;
        return ret;
    };
    imports.wbg.__wbg_deleteContents_b5fa9b266df5a846 = function() { return handleError(function (arg0) {
        arg0.deleteContents();
    }, arguments) };
    imports.wbg.__wbg_deltaY_54b262622880de65 = function(arg0) {
        const ret = arg0.deltaY;
        return ret;
    };
    imports.wbg.__wbg_detail_0b93b79b496faac7 = function(arg0) {
        const ret = arg0.detail;
        return ret;
    };
    imports.wbg.__wbg_detail_4c10b6dbc55722e5 = function(arg0) {
        const ret = arg0.detail;
        return ret;
    };
    imports.wbg.__wbg_dispatchEvent_1e92fa61005ad17d = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.dispatchEvent(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_documentElement_246c92e988ccf7aa = function(arg0) {
        const ret = arg0.documentElement;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_document_ccab1280c39d8474 = function(arg0) {
        const ret = arg0.document;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_done_b0c87979ae907ec7 = function(arg0) {
        const ret = arg0.done;
        return ret;
    };
    imports.wbg.__wbg_downloadappupdate_7551f6218f8f508b = function() { return handleError(function (arg0) {
        const ret = download_app_update(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_elementFromPoint_93f2392b18b25a21 = function(arg0, arg1, arg2) {
        const ret = arg0.elementFromPoint(arg1, arg2);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_enqueue_a0dbe37f5e0b5194 = function() { return handleError(function (arg0, arg1) {
        arg0.enqueue(arg1);
    }, arguments) };
    imports.wbg.__wbg_entries_ad16e471dc2ff55d = function(arg0) {
        const ret = Object.entries(arg0);
        return ret;
    };
    imports.wbg.__wbg_error_7534b8e9a36f1ab4 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        if (arg0 !== 0) { wasm.__wbindgen_free(arg0, arg1, 1); }
        console.error(v0);
    };
    imports.wbg.__wbg_error_f6b279d381a3b2fe = function(arg0) {
        console.error(arg0);
    };
    imports.wbg.__wbg_files_a248bbe9582d5573 = function(arg0) {
        const ret = arg0.files;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_files_c347b0257cfdd7f1 = function(arg0) {
        const ret = arg0.files;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_focus_cd53f6484acff5f9 = function() { return handleError(function (arg0) {
        arg0.focus();
    }, arguments) };
    imports.wbg.__wbg_forcechatscrollbottom_99f096f31e32543e = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        force_chat_scroll_bottom(v0);
    };
    imports.wbg.__wbg_getAttribute_1ba72d0387ead132 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        const ret = arg1.getAttribute(v0);
        var ptr2 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
    };
    imports.wbg.__wbg_getBoundingClientRect_5b5c4b8f0c9a1e20 = function(arg0) {
        const ret = arg0.getBoundingClientRect();
        return ret;
    };
    imports.wbg.__wbg_getData_8830580654e128f1 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        const ret = arg1.getData(v0);
        const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
    }, arguments) };
    imports.wbg.__wbg_getDate_b22c7048ae20a7b9 = function(arg0) {
        const ret = arg0.getDate();
        return ret;
    };
    imports.wbg.__wbg_getElementById_33fc85153437dfc2 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.getElementById(v0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_getFullYear_3ef5e010ca7251cb = function(arg0) {
        const ret = arg0.getFullYear();
        return ret;
    };
    imports.wbg.__wbg_getHours_352c89fb5d19b025 = function(arg0) {
        const ret = arg0.getHours();
        return ret;
    };
    imports.wbg.__wbg_getItem_6ac3689c261f3271 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        const ret = arg1.getItem(v0);
        var ptr2 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
    }, arguments) };
    imports.wbg.__wbg_getMinutes_33ca0c08c82306ea = function(arg0) {
        const ret = arg0.getMinutes();
        return ret;
    };
    imports.wbg.__wbg_getMonth_16abefd1fc9ca88f = function(arg0) {
        const ret = arg0.getMonth();
        return ret;
    };
    imports.wbg.__wbg_getSeconds_eddac3c5a354d759 = function(arg0) {
        const ret = arg0.getSeconds();
        return ret;
    };
    imports.wbg.__wbg_getSelection_9c6de0830d0ff1a1 = function() { return handleError(function (arg0) {
        const ret = arg0.getSelection();
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_get_a052ead7ce021b7c = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_get_c067c904c6ae703b = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_get_eaa3e583c50cc093 = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return ret;
    };
    imports.wbg.__wbg_get_fcdda07c7811211b = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function(arg0, arg1) {
        const ret = arg0[arg1];
        return ret;
    };
    imports.wbg.__wbg_globalThis_13e2993efe03d1e5 = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_global_24aa43240b06bb32 = function() { return handleError(function () {
        const ret = global.global;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_hasAttribute_97caab9ad64d7cbe = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.hasAttribute(v0);
        return ret;
    };
    imports.wbg.__wbg_has_bda69115f027fa31 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_height_a996d194aad5783b = function(arg0) {
        const ret = arg0.height;
        return ret;
    };
    imports.wbg.__wbg_highlightbyid_efa09d252d833028 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = highlight_by_id(v0);
        return ret;
    };
    imports.wbg.__wbg_highlightsetcode_15424e28c6be72e0 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        const ret = highlight_set_code(v0, v1);
        return ret;
    };
    imports.wbg.__wbg_host_d6ede0b221292091 = function(arg0) {
        const ret = arg0.host;
        return ret;
    };
    imports.wbg.__wbg_importmotifdnafile_05b85d7f133c5e5a = function() { return handleError(function (arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = import_motif_dna_file(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_innerHeight_1e4921e8277cc2e6 = function() { return handleError(function (arg0) {
        const ret = arg0.innerHeight;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_innerWidth_cd40c3c28a697fe8 = function() { return handleError(function (arg0) {
        const ret = arg0.innerWidth;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_inputType_995d2f2484053082 = function(arg0, arg1) {
        const ret = arg1.inputType;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_515f4988c901c6a3 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_CustomEvent_f0bac1364a2a0e18 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof CustomEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Element_e94aa9705d1803f3 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Element;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlElement_3365e609ba287ef0 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlInputElement_4c27427dbb941c85 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLInputElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlSelectElement_e0272e3dbc133a68 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLSelectElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlTableElement_6ae45945a4bca0fe = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLTableElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlTableRowElement_f742b49b51579fb2 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLTableRowElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlTextAreaElement_fd87bb1acdccc62c = function(arg0) {
        let result;
        try {
            result = arg0 instanceof HTMLTextAreaElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_InputEvent_fe8abf391056db70 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof InputEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_KeyboardEvent_f1dd1f8f7c6c8771 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof KeyboardEvent;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Map_e67e7498d1e12e28 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Map;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Node_5d31b3d156a8805d = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Node;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_ShadowRoot_b96cd24e0c6e72cf = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ShadowRoot;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_eaeadd4d6c34c4ac = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_53c1828409efdb62 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_invoke_11d08fc1de0d02ce = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = invoke(v0, arg2);
        return ret;
    };
    imports.wbg.__wbg_invokestrict_362c375227e99b63 = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = invoke_strict(v0, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_invoketimeout_a1df8560e873af12 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = invoke_timeout(v0, arg2, arg3 >>> 0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_isArray_5da94c9a416f8b07 = function(arg0) {
        const ret = Array.isArray(arg0);
        return ret;
    };
    imports.wbg.__wbg_isCollapsed_8d9ac5c1c3ed4655 = function(arg0) {
        const ret = arg0.isCollapsed;
        return ret;
    };
    imports.wbg.__wbg_isComposing_2e509c7550f16924 = function(arg0) {
        const ret = arg0.isComposing;
        return ret;
    };
    imports.wbg.__wbg_isSafeInteger_2696e3355dca53d5 = function(arg0) {
        const ret = Number.isSafeInteger(arg0);
        return ret;
    };
    imports.wbg.__wbg_is_3016d993c6b11c8c = function(arg0, arg1) {
        const ret = Object.is(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbg_item_59b878641c45d507 = function(arg0, arg1) {
        const ret = arg0.item(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_item_f4dccc4de2bcae1b = function(arg0, arg1) {
        const ret = arg0.item(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_iterator_9d8d34fc3b3478fa = function() {
        const ret = Symbol.iterator;
        return ret;
    };
    imports.wbg.__wbg_jumpchatscroll_32fa0a3a845f55f5 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        jump_chat_scroll(v0, v1);
    };
    imports.wbg.__wbg_keyCode_f19e71fd131dcaa3 = function(arg0) {
        const ret = arg0.keyCode;
        return ret;
    };
    imports.wbg.__wbg_key_dea8cf79b39c2b85 = function(arg0, arg1) {
        const ret = arg1.key;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_language_f7b2596a801d2555 = function(arg0, arg1) {
        const ret = arg1.language;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_left_88ad568f55e20640 = function(arg0) {
        const ret = arg0.left;
        return ret;
    };
    imports.wbg.__wbg_length_0758bcf10edcc827 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_2b9d45739e1dd005 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_315b4882b25605db = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_e0a8d18aa5a5b9cd = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_listen_09d6687600604318 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = listen(v0, arg2);
        return ret;
    };
    imports.wbg.__wbg_listencurrentwindow_d72a819b6e24a87c = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = listen_current_window(v0, arg2);
        return ret;
    };
    imports.wbg.__wbg_localStorage_dfcd8043aa25b085 = function() { return handleError(function (arg0) {
        const ret = arg0.localStorage;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_location_7a16fd6e0a15728d = function(arg0) {
        const ret = arg0.location;
        return ret;
    };
    imports.wbg.__wbg_log_48418cd28af83bc1 = function(arg0) {
        console.log(arg0);
    };
    imports.wbg.__wbg_mediathumbnailurl_033f9d05099926e5 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = media_thumbnail_url(v0);
        return ret;
    };
    imports.wbg.__wbg_mediaurl_ccaa6d51a10f4385 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = media_url(v0);
        return ret;
    };
    imports.wbg.__wbg_metaKey_133f0b3d09e31dc5 = function(arg0) {
        const ret = arg0.metaKey;
        return ret;
    };
    imports.wbg.__wbg_mountmcpapp_6a0d8a2e6712f69a = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        var v2 = getCachedStringFromWasm0(arg4, arg5);
        const ret = mount_mcp_app(v0, v1, v2);
        return ret;
    };
    imports.wbg.__wbg_mountpreview_c174bc9722d0ff06 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        var v2 = getCachedStringFromWasm0(arg4, arg5);
        const ret = mount_preview(v0, v1, v2);
        return ret;
    };
    imports.wbg.__wbg_mountterminal_c58b150efdc2ef97 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        mount_terminal(v0, v1);
    };
    imports.wbg.__wbg_namespaceURI_440e52ebb0c9d836 = function(arg0, arg1) {
        const ret = arg1.namespaceURI;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_navigator_8c0f976c02bfd738 = function(arg0) {
        const ret = arg0.navigator;
        return ret;
    };
    imports.wbg.__wbg_new0_8b12823831e154bd = function() {
        const ret = new Date();
        return ret;
    };
    imports.wbg.__wbg_new_088c2aed93e4a66e = function(arg0) {
        const ret = new Date(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_4d633ef144d63fc8 = function() {
        const ret = new Object();
        return ret;
    };
    imports.wbg.__wbg_new_70746da30d76ff97 = function() {
        const ret = new Array();
        return ret;
    };
    imports.wbg.__wbg_new_8a6f238a6ece86ea = function() {
        const ret = new Error();
        return ret;
    };
    imports.wbg.__wbg_new_b6300ed395abe7eb = function() { return handleError(function () {
        const ret = new Range();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_b640a3f9c6328a67 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = new Error(v0);
        return ret;
    };
    imports.wbg.__wbg_new_c688c3dc071c5402 = function(arg0) {
        const ret = new Uint8Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_cbe0c38d741bf41b = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_653(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return ret;
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_db3ea6cf623aa9e9 = function() {
        const ret = new Map();
        return ret;
    };
    imports.wbg.__wbg_newnoargs_34d683ae578a9146 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = new Function(v0);
        return ret;
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_673a3aeb07c2ac84 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwitheventinitdict_6f831fba17789cba = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = new CustomEvent(v0, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_nextSibling_5a9ad4c05bedf357 = function(arg0) {
        const ret = arg0.nextSibling;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_next_01ea8e3d39694078 = function(arg0) {
        const ret = arg0.next;
        return ret;
    };
    imports.wbg.__wbg_next_9f06bda8fc7eb007 = function() { return handleError(function (arg0) {
        const ret = arg0.next();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_notifychatscroll_29e286178eaea1a4 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        notify_chat_scroll(v0);
    };
    imports.wbg.__wbg_now_06df7b03d092fcb1 = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_offsetHeight_85fe23604fad02cc = function(arg0) {
        const ret = arg0.offsetHeight;
        return ret;
    };
    imports.wbg.__wbg_offsetLeft_25bc388d27210423 = function(arg0) {
        const ret = arg0.offsetLeft;
        return ret;
    };
    imports.wbg.__wbg_offsetWidth_0645725d2ac6c385 = function(arg0) {
        const ret = arg0.offsetWidth;
        return ret;
    };
    imports.wbg.__wbg_outerHTML_4f9892d5a507d27b = function(arg0, arg1) {
        const ret = arg1.outerHTML;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_parentElement_3535f47ded148835 = function(arg0) {
        const ret = arg0.parentElement;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_parentNode_701d265215416746 = function(arg0) {
        const ret = arg0.parentNode;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_parkmcpapp_1cd9c47c39513128 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        park_mcp_app(v0);
    };
    imports.wbg.__wbg_parse_c4ca7ce01352f571 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = Date.parse(v0);
        return ret;
    };
    imports.wbg.__wbg_pointerId_2a1aceb24ff9bf62 = function(arg0) {
        const ret = arg0.pointerId;
        return ret;
    };
    imports.wbg.__wbg_preservechatscrollonprepend_0a53ec92f15c8ddb = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        preserve_chat_scroll_on_prepend(v0, v1);
    };
    imports.wbg.__wbg_preventDefault_a0d7b368003b2ecb = function(arg0) {
        arg0.preventDefault();
    };
    imports.wbg.__wbg_previewselection_49eae1a254e8b06d = function(arg0, arg1, arg2) {
        const ret = preview_selection(arg1, arg2);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_previousSibling_4a9475c6ea85782b = function(arg0) {
        const ret = arg0.previousSibling;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_querySelectorAll_e64f5ffa42d7428d = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.querySelectorAll(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_querySelector_62ef78a32932ffb1 = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.querySelector(v0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_querySelector_cf9916f5b8d16721 = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.querySelector(v0);
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    }, arguments) };
    imports.wbg.__wbg_queueMicrotask_5a8a9131f3f0b37b = function(arg0) {
        const ret = arg0.queueMicrotask;
        return ret;
    };
    imports.wbg.__wbg_queueMicrotask_6d79674585219521 = function(arg0) {
        queueMicrotask(arg0);
    };
    imports.wbg.__wbg_random_183700195e6eacc1 = function() {
        const ret = Math.random();
        return ret;
    };
    imports.wbg.__wbg_readfeedbackinputfiles_ec460007319f87d9 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = read_feedback_input_files(v0);
        return ret;
    };
    imports.wbg.__wbg_releasePointerCapture_6938f843bcaf32a8 = function() { return handleError(function (arg0, arg1) {
        arg0.releasePointerCapture(arg1);
    }, arguments) };
    imports.wbg.__wbg_removeAttribute_062132563ed0e29e = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.removeAttribute(v0);
    }, arguments) };
    imports.wbg.__wbg_removeChild_b36a224cbd948b07 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.removeChild(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_bb644b1a75cd35de = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.removeEventListener(v0, arg3, arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_ef52e5ffbdd04c77 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.removeEventListener(v0, arg3);
    }, arguments) };
    imports.wbg.__wbg_removeItem_66728f34429cebde = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.removeItem(v0);
    }, arguments) };
    imports.wbg.__wbg_removeProperty_083554cd5b03537e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        const ret = arg1.removeProperty(v0);
        const ptr2 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len2, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr2, true);
    }, arguments) };
    imports.wbg.__wbg_remove_68d9d5e4adc3573d = function() { return handleError(function (arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.remove(v0);
    }, arguments) };
    imports.wbg.__wbg_remove_ee9ae56850c16ded = function(arg0) {
        arg0.remove();
    };
    imports.wbg.__wbg_rendersharepng_d5e9f90eaebd6b17 = function() { return handleError(function (arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = render_share_png(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_requestAnimationFrame_ca335358b9db95f1 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.requestAnimationFrame(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_requestmotifselection_26b6e98b5fac5ac9 = function() { return handleError(function (arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = request_motif_selection(v0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_resolve_3a80274d7f39c97d = function(arg0) {
        const ret = Promise.resolve(arg0);
        return ret;
    };
    imports.wbg.__wbg_respond_2ea620e1ae191c68 = function() { return handleError(function (arg0, arg1) {
        arg0.respond(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_revealsavedmark_3be7ba7b61eb7177 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        reveal_saved_mark(v0);
    };
    imports.wbg.__wbg_right_95f45fc20081f3fd = function(arg0) {
        const ret = arg0.right;
        return ret;
    };
    imports.wbg.__wbg_rows_c70d99ee224d1a98 = function(arg0) {
        const ret = arg0.rows;
        return ret;
    };
    imports.wbg.__wbg_scrollHeight_2c14bdee2b31b44b = function(arg0) {
        const ret = arg0.scrollHeight;
        return ret;
    };
    imports.wbg.__wbg_scrollIntoView_14de36c92741f09d = function(arg0) {
        arg0.scrollIntoView();
    };
    imports.wbg.__wbg_scrollIntoView_c28cfdddc3fb3e95 = function(arg0, arg1) {
        arg0.scrollIntoView(arg1 !== 0);
    };
    imports.wbg.__wbg_scrollLeft_7d3931259d7b0398 = function(arg0) {
        const ret = arg0.scrollLeft;
        return ret;
    };
    imports.wbg.__wbg_scrollTop_4af0460b8d57aa3e = function(arg0) {
        const ret = arg0.scrollTop;
        return ret;
    };
    imports.wbg.__wbg_scrollWidth_9f91eacadaf61318 = function(arg0) {
        const ret = arg0.scrollWidth;
        return ret;
    };
    imports.wbg.__wbg_search_44aea68f650d418e = function() { return handleError(function (arg0, arg1) {
        const ret = arg1.search;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_select_3e3dbc98fcca1d8a = function(arg0) {
        arg0.select();
    };
    imports.wbg.__wbg_select_a26c1c5b6d7e4f48 = function(arg0) {
        arg0.select();
    };
    imports.wbg.__wbg_selectionEnd_19965fc65d30fb58 = function() { return handleError(function (arg0) {
        const ret = arg0.selectionEnd;
        return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
    }, arguments) };
    imports.wbg.__wbg_selectionStart_80e60b6a7f534df9 = function() { return handleError(function (arg0) {
        const ret = arg0.selectionStart;
        return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
    }, arguments) };
    imports.wbg.__wbg_self_526d3c066ffbe4bb = function() { return handleError(function () {
        const ret = self.self;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setAttribute_95bc271929fb3190 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        arg0.setAttribute(v0, v1);
    }, arguments) };
    imports.wbg.__wbg_setData_fa499f8fd9e3e843 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        arg0.setData(v0, v1);
    }, arguments) };
    imports.wbg.__wbg_setEndBefore_bd28bf8782be9c2d = function() { return handleError(function (arg0, arg1) {
        arg0.setEndBefore(arg1);
    }, arguments) };
    imports.wbg.__wbg_setInterval_9fc65c5404d6605a = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.setInterval(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setItem_ce98824fb9aa26be = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        arg0.setItem(v0, v1);
    }, arguments) };
    imports.wbg.__wbg_setPointerCapture_ba29c30e615cf847 = function() { return handleError(function (arg0, arg1) {
        arg0.setPointerCapture(arg1);
    }, arguments) };
    imports.wbg.__wbg_setProperty_07cd876e7959fdd5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        arg0.setProperty(v0, v1);
    }, arguments) };
    imports.wbg.__wbg_setSelectionRange_6963c34e42e41156 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.setSelectionRange(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_setStartAfter_a1fe3ba67d07b5d0 = function() { return handleError(function (arg0, arg1) {
        arg0.setStartAfter(arg1);
    }, arguments) };
    imports.wbg.__wbg_setTimeout_6f04d16ea09e54c7 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.setTimeout(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_000638896bde6661 = function(arg0, arg1, arg2) {
        arg0[arg1 >>> 0] = arg2;
    };
    imports.wbg.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
        arg0[arg1] = arg2;
    };
    imports.wbg.__wbg_set_67a14358fb2ec675 = function(arg0, arg1, arg2) {
        arg0.set(arg1, arg2 >>> 0);
    };
    imports.wbg.__wbg_set_8b8e3f1b0cdd7c13 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(arg0, arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_95e7d1054a357dde = function(arg0, arg1, arg2) {
        const ret = arg0.set(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_setchecked_3450943a7f3171a3 = function(arg0, arg1) {
        arg0.checked = arg1 !== 0;
    };
    imports.wbg.__wbg_setclassName_7fed0b50c8b2ef5e = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.className = v0;
    };
    imports.wbg.__wbg_setdata_3b036ac2553386d4 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.data = v0;
    };
    imports.wbg.__wbg_setdetail_515695766135e804 = function(arg0, arg1) {
        arg0.detail = arg1;
    };
    imports.wbg.__wbg_setdropEffect_89ec1541fbf080dc = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.dropEffect = v0;
    };
    imports.wbg.__wbg_seteffectAllowed_3068d3ec41262027 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.effectAllowed = v0;
    };
    imports.wbg.__wbg_setid_b197dd0c95c3ccc3 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.id = v0;
    };
    imports.wbg.__wbg_setinnerHTML_216dcfa0b115f204 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.innerHTML = v0;
    };
    imports.wbg.__wbg_setsavedmarks_b55476c2dbb5d1f8 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        set_saved_marks(v0);
    };
    imports.wbg.__wbg_setscrollLeft_8da07a1d3b16ae29 = function(arg0, arg1) {
        arg0.scrollLeft = arg1;
    };
    imports.wbg.__wbg_setscrollTop_d715feedefeef008 = function(arg0, arg1) {
        arg0.scrollTop = arg1;
    };
    imports.wbg.__wbg_setterminalactive_a4420824856eee85 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        set_terminal_active(v0, arg2 !== 0);
    };
    imports.wbg.__wbg_settextContent_d0aa95f9479ad4de = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.textContent = v0;
    };
    imports.wbg.__wbg_setvalue_be33ec46e22a4077 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        arg0.value = v0;
    };
    imports.wbg.__wbg_setwindowtitle_5b96dac093e4e6af = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = set_window_title(v0);
        return ret;
    };
    imports.wbg.__wbg_shiftKey_e78c25a896080017 = function(arg0) {
        const ret = arg0.shiftKey;
        return ret;
    };
    imports.wbg.__wbg_snapshotsharetheme_7618349483b01c00 = function(arg0) {
        const ret = snapshot_share_theme();
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_stack_0ed75d68575b0f3c = function(arg0, arg1) {
        const ret = arg1.stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_stopPropagation_dec556b6f6ff41e8 = function(arg0) {
        arg0.stopPropagation();
    };
    imports.wbg.__wbg_style_fef53785de1dcaf9 = function(arg0) {
        const ret = arg0.style;
        return ret;
    };
    imports.wbg.__wbg_switchchatscroll_f8d4d917688230eb = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        switch_chat_scroll(v0, v1);
    };
    imports.wbg.__wbg_syncpet_740a680090c6da34 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        sync_pet(v0, v1);
    };
    imports.wbg.__wbg_tagName_feaf3f73a1e6fefc = function(arg0, arg1) {
        const ret = arg1.tagName;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_target_378fe550d77512e9 = function(arg0) {
        const ret = arg0.target;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_textContent_de303c576e2e27a4 = function(arg0, arg1) {
        const ret = arg1.textContent;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_text_1173509ef434d6e8 = function(arg0) {
        const ret = arg0.text();
        return ret;
    };
    imports.wbg.__wbg_then_6d3dc850809c8deb = function(arg0, arg1, arg2) {
        const ret = arg0.then(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_then_b7c2f95cea97f6f7 = function(arg0, arg1) {
        const ret = arg0.then(arg1);
        return ret;
    };
    imports.wbg.__wbg_timeStamp_581b56114f71a070 = function(arg0) {
        const ret = arg0.timeStamp;
        return ret;
    };
    imports.wbg.__wbg_toISOString_075a5ee4c9bfab22 = function(arg0) {
        const ret = arg0.toISOString();
        return ret;
    };
    imports.wbg.__wbg_toLocaleDateString_e3cea59a4629e9c8 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.toLocaleDateString(v0, arg3);
        return ret;
    };
    imports.wbg.__wbg_toLocaleString_d66aebd87d460144 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.toLocaleString(v0, arg3);
        return ret;
    };
    imports.wbg.__wbg_toString_cce7486657636439 = function(arg0) {
        const ret = arg0.toString();
        return ret;
    };
    imports.wbg.__wbg_top_46231f6eff9f77c2 = function(arg0) {
        const ret = arg0.top;
        return ret;
    };
    imports.wbg.__wbg_unmountterminal_7c0fc4eaa5e7ceed = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        unmount_terminal(v0);
    };
    imports.wbg.__wbg_uploadinputfiles_cd6ed72f4e7e02c0 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = upload_input_files(v0);
        return ret;
    };
    imports.wbg.__wbg_value_1df9703635307b9c = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_585a85abbbbb7fed = function(arg0) {
        const ret = arg0.value;
        return ret;
    };
    imports.wbg.__wbg_value_7abc125f9f326665 = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_9faf4bfe4b9ad832 = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_view_f07a306a6e7c1ca7 = function(arg0) {
        const ret = arg0.view;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_warn_5207af46e8829e9f = function(arg0) {
        console.warn(arg0);
    };
    imports.wbg.__wbg_width_05ebc55860df4924 = function(arg0) {
        const ret = arg0.width;
        return ret;
    };
    imports.wbg.__wbg_window_85593c44ba353d6d = function() { return handleError(function () {
        const ret = window.window;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_windowcontrol_83d4ea4b33f39564 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        const ret = window_control(v0);
        return ret;
    };
    imports.wbg.__wbg_writeText_36b625fa215f50c4 = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        const ret = arg0.writeText(v0);
        return ret;
    };
    imports.wbg.__wbg_x_68bc0f28104ceac6 = function(arg0) {
        const ret = arg0.x;
        return ret;
    };
    imports.wbg.__wbg_y_0a6b71308670dafa = function(arg0) {
        const ret = arg0.y;
        return ret;
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +arg0;
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i64 = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
        const ret = BigInt.asUintN(64, arg0);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = arg1;
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = arg0;
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = arg0.original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper102711 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 13893, __wbg_adapter_88);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper104161 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 13953, __wbg_adapter_91);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10449 = function(arg0, arg1, arg2) {
        const ret = makeClosure(arg0, arg1, 744, __wbg_adapter_58);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10451 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 746, __wbg_adapter_61);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10453 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 748, __wbg_adapter_64);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10455 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 750, __wbg_adapter_67);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10457 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 752, __wbg_adapter_70);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10459 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 754, __wbg_adapter_73);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10461 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 756, __wbg_adapter_76);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10463 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 758, __wbg_adapter_79);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10465 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 760, __wbg_adapter_82);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper10467 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 762, __wbg_adapter_85);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper108634 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 13961, __wbg_adapter_94);
        return ret;
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = arg0 in arg1;
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_2;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_is_bigint = function(arg0) {
        const ret = typeof(arg0) === 'bigint';
        return ret;
    };
    imports.wbg.__wbindgen_is_falsy = function(arg0) {
        const ret = !arg0;
        return ret;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = arg0 === null;
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = arg0;
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(arg0) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
        const ret = arg0 === arg1;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = arg0 == arg1;
        return ret;
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_rethrow = function(arg0) {
        throw arg0;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports['./snippets/superscience-ui-50d348c2a8229c6d/src/api.js'] = __wbg_star0;
    imports['./snippets/superscience-ui-50d348c2a8229c6d/src/context_menu.js'] = __wbg_star1;
    imports['./snippets/superscience-ui-50d348c2a8229c6d/src/marks.js'] = __wbg_star2;
    imports['./snippets/superscience-ui-50d348c2a8229c6d/src/scroll.js'] = __wbg_star3;

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('superscience-ui_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
