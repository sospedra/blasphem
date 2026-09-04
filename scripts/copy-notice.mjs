import { copyFileSync } from "node:fs";
import { resolve } from "node:path";

copyFileSync(new URL("../NOTICE", import.meta.url), resolve("NOTICE"));
