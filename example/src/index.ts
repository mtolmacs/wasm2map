import { util } from './util.js'

const OBJECTS = {
    __store: {},
    __head: 1,

    add: (obj) => {
        OBJECTS.__store[OBJECTS.__head++] = obj;
        return OBJECTS.__head;
    },
    remove: (idx) => delete OBJECTS.__store[idx],
    get: (idx) => OBJECTS.__store[idx],
};
const Mem = {
    __raw_memory: new WebAssembly.Memory({ initial: 1024 }),
    __raw_alloc: (size) => {
        console.error('No allocator was set up!')
        return 0
    },
    __raw_free: (pt, len) => console.error('No allocator was set up!'),

    alloc: (size): number => {
        return Mem.__raw_alloc(size);
    },
    free: (ptr, len) => {
        Mem.__raw_free(ptr, len);
    },
    drop: (ref) => OBJECTS.remove(ref),
};
const JSString = {
    __encoder: new TextEncoder(),
    __decoder: new TextDecoder(),

    from_wasm: (ptr) => {
        const len_data = new DataView(Mem.__raw_memory.buffer, ptr, 4);
        let len = len_data.getUint32(0, true);
        const buffer = new DataView(Mem.__raw_memory.buffer, ptr + 4, len);
        let str = JSString.__decoder.decode(buffer);
        Mem.free(ptr, 4 + len);
        return str;
    },
    to_wasm: (str) => {
        const string_data = JSString.__encoder.encode(str);
        const string_size = string_data.byteLength;
        const ptr = Mem.alloc(4 + string_size);
        const len_buffer = new DataView(Mem.__raw_memory.buffer, ptr, 4 + string_size);
        len_buffer.setUint32(0, string_size, true);
        let buffer = new Uint8Array(Mem.__raw_memory.buffer);
        buffer.set(string_data, ptr + 4);                
        return ptr;
    },
};
const Cons = {
    log: (ptr) => console.log(JSString.from_wasm(ptr)),
    warn: (ptr) => console.warn(JSString.from_wasm(ptr)),
    error: (ptr) => console.error(JSString.from_wasm(ptr)),
}
const importObject = {
    env: {
        property: (instance_idx, property_name) => {
            let res = OBJECTS.get(instance_idx)[property_name];
            return OBJECTS.add(res);
        },
    },
    memory: Mem,
    console: Cons,
    window: {
        default: () => OBJECTS.add(window),
    },
    document: {
        cookies: (document_instance) => { }
    },
};

window.addEventListener("load", () => {
    WebAssembly.instantiateStreaming(fetch('/example.wasm'), importObject as unknown as WebAssembly.Imports).then(results => {
        Mem.__raw_memory = results.instance.exports.memory as unknown as WebAssembly.Memory;
        Mem.__raw_alloc = results.instance.exports.alloc as unknown as (size) => number;
        Mem.__raw_free = results.instance.exports.free as unknown as (ptr, len) => void;

        try {
            //util();
            let start = results.instance.exports.start as () => Promise<void>;
            start().catch(_ => {});
        } catch {}
    });
});
