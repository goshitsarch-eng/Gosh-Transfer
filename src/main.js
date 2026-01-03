// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Frontend entry point

import "./styles/global.css";
import App from "./App.svelte";
import { mount } from "svelte";

const app = mount(App, {
  target: document.getElementById("app"),
});

export default app;
