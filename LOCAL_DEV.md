# Local Metro Dev Mode

Use this workflow to edit the React Metro UI with hot reload (no rebuild/deploy loop).

## 1) Run backend (terminal A)

```bash
cd /home/critbit/Projects/software/00_done/CringeCast2/CringeCast/rust-rewrite
CRINGECAST_ROOT=/home/critbit/Projects/software/00_done/CringeCast2/CringeCast \
CRINGECAST_PORT=42069 \
cargo run --bin cringecast-server
```

## 2) Run frontend dev server (terminal B)

```bash
cd /home/critbit/Projects/software/00_done/CringeCast2/CringeCast/frontend-react
npm run dev:local
```

Open: `http://localhost:5173/`

Vite proxies API endpoints (`/say`, `/mow`, `/guess`, `/play`, `/vol`, `/stop`, `/teapot`, `/getFilelist`, `/uploader`) to `http://127.0.0.1:42069`.

## 3) Build for deployed `/metro`

```bash
cd /home/critbit/Projects/software/00_done/CringeCast2/CringeCast/frontend-react
npm run build:metro
```

This writes production files into `CringeCast/static/metro`.
