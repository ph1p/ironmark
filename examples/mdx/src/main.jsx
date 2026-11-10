import { createRoot } from "react-dom/client";
import Guide from "./Guide.mdx";
import { components } from "./components.jsx";
import "./style.css";

createRoot(document.getElementById("root")).render(<Guide name="World" components={components} />);
