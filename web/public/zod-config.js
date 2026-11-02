// Classic script loaded before the app bundle (index.html). The server CSP forbids eval;
// without this, Zod 4 probes `new Function` at load time, which is reported as a CSP violation.
globalThis.__zod_globalConfig = Object.assign(globalThis.__zod_globalConfig || {}, { jitless: true });
