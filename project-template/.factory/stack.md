# Stack (read by every factory agent)
- UI library: HeroUI v3 (@heroui/react). Look components up via the `ui-kit` MCP.
- Styling: Tailwind v4 + HeroUI theme variables. No other component library.
- DB: Supabase (DEV project via `db` MCP; reviewers use `db-readonly`). Every new table needs RLS.
- Dev: `npm run dev -- --port 3000`
- Gates: `npm run typecheck`, `npm run lint`, `npm test`, `npm run build`
- Risk hot spots: supabase/migrations/, app/(auth)/, middleware.ts
