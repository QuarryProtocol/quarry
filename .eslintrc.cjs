// @ts-check

"use strict";

require("@rushstack/eslint-patch/modern-module-resolution");

/** @type import('eslint').Linter.Config */
module.exports = {
  root: true,
  parserOptions: {
    project: "tsconfig.json",
  },
  extends: ["@saberhq"],
  env: {
    node: true,
  },
};
