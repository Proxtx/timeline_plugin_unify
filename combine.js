const jobIdLength = 5;

export const genCombine = (module, genModule, server) => {
  return genModule(async (body) => {
    const jobId = randomString(jobIdLength);
    let resolve;
    let result;
    let listener = window.addEventListener("message", async (event) => {
      if (event.data.substring(0, 10 + jobIdLength) == "combine-ac" + jobId) {
        result = JSON.parse(event.data.substring(10 + jobIdLength));
        resolve();
      }
    });
    server.postMessage("combine-ts" + jobId + JSON.stringify(body), "*");
    await new Promise((r) => {
      resolve = r;
    });
    window.removeEventListener("message", listener);
    return result;
  }, module);
};

let chars = "abcdefghijklmnopqrstuvwxyz".split("");
let randomString = (length) => {
  let result = "";
  for (let i = 0; i < length; i++) {
    result += chars[random(0, chars.length - 1)];
  }
  return result;
};

export const random = (min, max) => {
  return Math.floor(Math.random() * (max - min + 1)) + min;
};

let asyncFunctionConstructor = (async () => {}).constructor;

/**
 * Generates a proxy object which represents the server.
 * @param {Function} request The function that handles the request
 * @param {String} module The module name. Optional if you only have one module
 * @returns A proxy objects which acts like "import * as allExports from server"
 */
export const genModule = async (request, module) => {
  let info = await request({ info: true, module });
  let exports = info.exports;
  if (!info.success)
    throw new Error(
      "Combine error. Server info was unsuccessful: " + JSON.stringify(info)
    );
  return new Proxy(
    {},
    {
      get: (target, p) => {
        if (!info || !exports[p]) return;
        let body;
        if (exports[p].function) {
          body = {
            export: p,
            module: module,
          };
          if (request instanceof asyncFunctionConstructor)
            return async function (...args) {
              body.arguments = args;
              return parseRes(await request(body));
            };
          return function (...args) {
            body.arguments = args;
            return parseRes(request(body));
          };
        }
        body = {
          export: p,
          module: module,
        };
        if (request instanceof asyncFunctionConstructor)
          return request(body).then((res) => {
            return parseRes(res);
          });
        return parseRes(request(body));
      },
      set: () => {
        return false;
      },
    }
  );
};

const parseRes = (res) => {
  if (res.success) {
    return res.data;
  }
  return res;
};
