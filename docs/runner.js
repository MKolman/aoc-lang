importScripts("./pkg/aoc_lang.js");
const aoc = wasm_bindgen;

function report(id, message, time, debug) {
    self.postMessage({ id, message, time, debug});

}
function parse({ data }) {
    let success = false;
    let message = "error";
    const startTime = performance.now();
    try {
        message = aoc.run(data.code, data.debug);
        success = true;
    } catch (e) {
        message = e;
    }
    const endTime = performance.now();
    report(success, message, endTime - startTime, data.debug);
}
async function init() {
    await aoc("./pkg/aoc_lang_bg.wasm");
    self.onmessage = parse;
    console.log("LOADED");
}
init();