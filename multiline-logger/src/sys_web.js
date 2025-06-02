// SPDX-License-Identifier: MIT OR Apache-2.0
// small js printing logic
const B = "font-weight:bold;color:", CY = B + "gold", CT = B + "teal", CG = B + "seagreen", CV = B + "palevioletred", CF = B + "firebrick"
const T = ["error", "warn", "info", "log", "debug", "red", "darkorange", "blue", "green", "deeppink", "Error", "Warn", "Info", "Debug", "Trace"]
/**@param{string}t@param{number}y@param{number}m@param{number}d*/
export function js_header(t, y, m, d) { console.log("%c== %s - %04d-%02d-%02d ==", CY, t, y, m, d) }
/**@param{number}y@param{number}m@param{number}d*/
export function js_new_day(y, m, d) { console.log("%c= %04d-%02d-%02d =", CY, y, m, d) }
/**@param{string}md@param{string|number}th@param{string}tx@param{number}li@param{number}lv@param{number}h@param{number}m@param{number}s@param{number}ms*/
export function js_record(md, th, tx, li, lv, h, m, s, ms) { console[T[lv]]("%c%s %c%02d:%02d:%02d.%03d %c%s %c%s\n%c%s", B + T[lv + 5], T[lv + 10], CT, h, m, s, ms, CG, li ? md + ":" + li : md, CV, th, "", tx) }
/**@param{string}tt@param{string|number}th@param{string}tx@param{string}lc@param{string?}tr*/
export function js_panic(tt, th, tx, lc, tr) { console.error("%c== %s - %c%s%c Panic ==\n%c%s\n%c→ %s%c%s", CF, tt, CV, th, CF, "", tx, CG, lc, "", tr) }
export function js_trace() { const stack = new Error().stack; return stack ? "\nBACKTRACE:\n" + stack : "" }
