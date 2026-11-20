import { createContext, useContext, useState, type ReactNode } from "react";

export type Route =
  | { type: "library"; kind: "text" | "image" | "files" | "link" | null; view: "all" | "favorites" | "pinned" | "recent" }
  | { type: "settings" };

interface NavContextValue {
  route: Route;
  navigate: (route: Route) => void;
  settingsTab: string;
  setSettingsTab: (tab: string) => void;
}

const NavContext = createContext<NavContextValue | null>(null);

export function useNavigate() {
  const ctx = useContext(NavContext);
  if (!ctx) throw new Error("navigation outside provider");
  return ctx.navigate;
}

export function useNav() {
  const ctx = useContext(NavContext);
  if (!ctx) throw new Error("navigation outside provider");
  return ctx;
}

export function NavProvider({ children }: { children: ReactNode }) {
  const [route, setRoute] = useState<Route>({ type: "library", kind: null, view: "all" });
  const [settingsTab, setSettingsTab] = useState("general");

  return (
    <NavContext.Provider value={{ route, navigate: setRoute, settingsTab, setSettingsTab }}>
      {children}
    </NavContext.Provider>
  );
}
