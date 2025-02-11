import React from 'react';
import { createRoot } from "react-dom/client";

import App from "./App";

const wrapper = document.getElementById("container");
const root = createRoot(wrapper!);
root.render(<App />);
