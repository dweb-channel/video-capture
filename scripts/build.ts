import fs from "node:fs";
import url from "node:url";
import { $ } from "./$.ts";

const name = "example-wasm"

const resolve = (path: string) => url.fileURLToPath(import.meta.resolve(path));
const package_json_filepath = resolve("../pkg/package.json");
const version =
  Deno.args[0] ??
  (() => {
    try {
      return JSON.parse(fs.readFileSync(package_json_filepath, "utf-8"))
        .version;
    } catch {
      return "0.1.0";
    }
  })();

await $("wasm-pack", "build  --target web --dev --scope dweb-browser");

fs.renameSync(resolve(`../pkg/${name}.js`), resolve(`../pkg/${name}.mjs`));

// await $("wasm-pack", "build  --target nodejs --release --scope dweb-browser");


const packageJson = JSON.parse(fs.readFileSync(package_json_filepath, "utf-8"));
const write_bg_wasm_file = (BG_WASM_BASE64: string) => {
  const bg_wasm_ts_filepath = resolve("../pkg/${name}_bg_wasm.ts");
  fs.writeFileSync(
    bg_wasm_ts_filepath,
    fs
      .readFileSync(resolve(`./template/${name}_bg_wasm.ts`), "utf8")
      .replace("BG_WASM_BASE64", BG_WASM_BASE64)
  );
};
write_bg_wasm_file(
  fs.readFileSync(resolve(`../pkg/${name}_bg.wasm`), "base64")
);

$.cd(resolve("../pkg"));

await $(
  "esbuild",
  `${name}_bg_wasm.ts --format=esm --outfile=${name}_bg_wasm.mjs`
);
await $(
  "esbuild",
  `${name}_bg_wasm.ts --format=cjs --outfile=${name}_bg_wasm.js`
);

write_bg_wasm_file("");

Object.assign(packageJson, {
  files: [`${name}.mjs`,...packageJson.files,`${name}_bg_wasm.mjs`, `${name}_bg_wasm.js`, `${name}_bg_wasm.ts`].sort(),
  version: Deno.args[0] ?? version,
  exports: {
    ".": {
      import: `./${name}.mjs`,
      types: `./${name}.d.ts`,
      require:`./${name}.js`
    },
    "./${name}_bg.wasm": `./${name}_bg.wasm`,
    "./${name}_bg_wasm.ts": {
      types: `./${name}_bg_wasm.ts`,
      import: `./${name}_bg_wasm.mjs`,
      require: `./${name}_bg_wasm.js`,
    },
    "./${name}_bg_wasm": {
      import: `./${name}_bg_wasm.mjs`,
      types: `./${name}_bg_wasm.ts`,
      require:`./${name}_bg_wasm.js`
    },
  },
  repository: {
    type: "git",
    url: "git://github.com/BioforestChain/svg-wasm.git",
  },
  bugs: {
    email: "waterbang6@gmail.com",
    url: "https://github.com/BioforestChain/svg-wasm/issues",
  },
  author: "waterbang <waterbang6@gmail.com>",
  license: "MIT",
  keywords: ["svg", "png", "webp"],
});

fs.writeFileSync(package_json_filepath, JSON.stringify(packageJson, null, 2));
