// SPDX-License-Identifier: AGPL-3.0
// Theme management for Gosh Transfer

// Media query for system preference
const systemDarkQuery = window.matchMedia("(prefers-color-scheme: dark)");

/**
 * Apply theme to the document root
 * @param {"dark" | "light" | "system"} theme - Theme preference
 */
export function applyTheme(theme) {
  let effectiveTheme;

  if (theme === "system") {
    effectiveTheme = systemDarkQuery.matches ? "dark" : "light";
  } else {
    effectiveTheme = theme;
  }

  document.documentElement.setAttribute("data-theme", effectiveTheme);
}

/**
 * Initialize theme and set up system preference listener
 * @param {"dark" | "light" | "system"} initialTheme - Initial theme preference
 * @returns {() => void} Cleanup function to remove listener
 */
export function initTheme(initialTheme) {
  // Apply initial theme
  applyTheme(initialTheme);

  // Listen for system preference changes (only matters when theme is "system")
  let currentTheme = initialTheme;

  const handleSystemChange = () => {
    if (currentTheme === "system") {
      applyTheme("system");
    }
  };

  systemDarkQuery.addEventListener("change", handleSystemChange);

  // Return cleanup function and a way to update the tracked theme
  return {
    cleanup: () => systemDarkQuery.removeEventListener("change", handleSystemChange),
    setTheme: (theme) => {
      currentTheme = theme;
      applyTheme(theme);
    },
  };
}
