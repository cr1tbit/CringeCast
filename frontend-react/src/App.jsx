import { useEffect, useMemo, useRef, useState } from "react";
import "./App.css";

const COLORS = ["bg-cyan", "bg-magenta", "bg-lime", "bg-teal", "bg-orange", "bg-darkViolet"];
const GRID_ROWS = 5;
const CONTROL_COLS = 6;
const LONG_TILE_THRESHOLD = 15;
const GDANSK = { lat: 54.352, lon: 18.6466 };

function weatherCodeLabel(code) {
  if (code === 0) return "Clear";
  if (code === 1) return "Mainly clear";
  if (code === 2) return "Partly cloudy";
  if (code === 3) return "Cloudy";
  if (code === 45 || code === 48) return "Fog";
  if ([51, 53, 55, 56, 57].includes(code)) return "Drizzle";
  if ([61, 63, 65, 66, 67, 80, 81, 82].includes(code)) return "Rain";
  if ([71, 73, 75, 77, 85, 86].includes(code)) return "Snow";
  if ([95, 96, 99].includes(code)) return "Thunderstorm";
  return "Weather";
}

function gridSizeStyle(cols, rows) {
  return {
    width: `calc(${cols} * var(--tile-size) + (${cols} - 1) * var(--grid-gap))`,
    height: `calc(${rows} * var(--tile-size) + (${rows} - 1) * var(--grid-gap))`,
  };
}

function tilePosStyle(x, y, w = 1, h = 1) {
  return {
    left: `calc(${x} * (var(--tile-size) + var(--grid-gap)))`,
    top: `calc(${y} * (var(--tile-size) + var(--grid-gap)))`,
    width: `calc(${w} * var(--tile-size) + (${w} - 1) * var(--grid-gap))`,
    height: `calc(${h} * var(--tile-size) + (${h} - 1) * var(--grid-gap))`,
  };
}

function useApi() {
  const request = async (path, options) => {
    try {
      const res = await fetch(path, options);
      const text = await res.text();
      return { res, text };
    } catch (e) {
      return null;
    }
  };

  return { request };
}

function layoutCategory(entries) {
  const used = new Set();
  const placed = [];
  let col = 0;

  for (const name of entries) {
    const w = name.length > LONG_TILE_THRESHOLD ? 2 : 1;
    let done = false;

    while (!done) {
      for (let row = 0; row < GRID_ROWS; row += 1) {
        let fits = true;
        for (let dx = 0; dx < w; dx += 1) {
          if (used.has(`${col + dx}:${row}`)) {
            fits = false;
            break;
          }
        }

        if (!fits) {
          continue;
        }

        for (let dx = 0; dx < w; dx += 1) {
          used.add(`${col + dx}:${row}`);
        }
        placed.push({ name, col, row, w });
        done = true;
        break;
      }

      if (!done) {
        col += 1;
      }
    }
  }

  const cols = Math.max(1, placed.reduce((max, it) => Math.max(max, it.col + it.w), 0));
  return { placed, cols };
}

function App() {
  const { request } = useApi();
  const scrollRef = useRef(null);
  const [parallaxX, setParallaxX] = useState(0);
  const [say, setSay] = useState("");
  const [mow, setMow] = useState("");
  const [guess, setGuess] = useState("");
  const [volume, setVolume] = useState(50);
  const [files, setFiles] = useState({});
  const [teapotEnabled, setTeapotEnabled] = useState(false);
  const [teapotKnown, setTeapotKnown] = useState(false);
  const [teapotRemainingSeconds, setTeapotRemainingSeconds] = useState(0);
  const [weather, setWeather] = useState({
    temp: null,
    code: null,
    max: null,
    min: null,
    ready: false,
  });

  const categories = useMemo(() => Object.keys(files).sort(), [files]);

  const formatRemaining = (seconds) => {
    const s = Math.max(0, seconds | 0);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) {
      return `${h}h ${m}m ${sec}s`;
    }
    if (m > 0) {
      return `${m}m ${sec}s`;
    }
    return `${sec}s`;
  };

  const refreshTeapotStatus = async () => {
    const out = await request("/teapot/status");
    if (out && out.res.status === 200) {
      try {
        const data = JSON.parse(out.text);
        setTeapotEnabled(Boolean(data.enabled));
        setTeapotRemainingSeconds(Number(data.remaining_seconds || 0));
        setTeapotKnown(true);
      } catch (_) {
        setTeapotEnabled(out.text.trim() === "enabled");
        setTeapotRemainingSeconds(0);
        setTeapotKnown(true);
      }
    }
  };

  useEffect(() => {
    const boot = async () => {
      const vol = await request("/vol");
      if (vol && vol.res.status === 200) {
        const parsed = parseInt(vol.text, 10);
        if (!Number.isNaN(parsed)) {
          setVolume(parsed);
        }
      }

      const list = await request("/getFilelist");
      if (list && list.res.status === 200) {
        try {
          setFiles(JSON.parse(list.text));
        } catch (_) {
          setFiles({});
        }
      }

      await refreshTeapotStatus();
    };

    boot();
  }, []);

  useEffect(() => {
    let active = true;

    const loadWeather = async () => {
      try {
        const url =
          `https://api.open-meteo.com/v1/forecast?latitude=${GDANSK.lat}&longitude=${GDANSK.lon}` +
          "&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min" +
          "&timezone=Europe%2FWarsaw&forecast_days=1";
        const res = await fetch(url);
        if (!res.ok) {
          return;
        }
        const data = await res.json();
        if (!active) {
          return;
        }
        setWeather({
          temp: Math.round(data?.current?.temperature_2m ?? 0),
          code: data?.current?.weather_code ?? null,
          max: Math.round(data?.daily?.temperature_2m_max?.[0] ?? 0),
          min: Math.round(data?.daily?.temperature_2m_min?.[0] ?? 0),
          ready: true,
        });
      } catch (_) {
      }
    };

    loadWeather();
    const timer = setInterval(loadWeather, 10 * 60 * 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) {
      return;
    }

    const onWheel = (event) => {
      if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) {
        return;
      }

      if (node.scrollWidth <= node.clientWidth) {
        return;
      }

      let dx = event.deltaY;
      if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) {
        dx *= 16;
      } else if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) {
        dx *= node.clientWidth;
      }

      node.scrollLeft += dx;
      event.preventDefault();
    };

    const onScroll = () => {
      setParallaxX(node.scrollLeft * 0.16);
    };

    onScroll();
    node.addEventListener("wheel", onWheel, { passive: false });
    node.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      node.removeEventListener("wheel", onWheel);
      node.removeEventListener("scroll", onScroll);
    };
  }, []);

  const uploadFile = async (file) => {
    const form = new FormData();
    form.append("file", file);
    const out = await request("/uploader", { method: "POST", body: form });
    if (out && out.res.status === 200) {
      const list = await request("/getFilelist");
      if (list && list.res.status === 200) {
        try {
          setFiles(JSON.parse(list.text));
        } catch (_) {
          setFiles({});
        }
      }
    }
  };

  return (
    <div className="metro-stage">
      <div
        className="parallax-bg"
        style={{ transform: `translate3d(${-parallaxX}px, 0, 0) scale(1.22)` }}
      />
      <div className="parallax-overlay" />
      <div className="metro-wrap fg-white" ref={scrollRef}>
      <section className="panel">
        <div className="panel-content">
          <div className="column section-block">
            <div className="column-head">
              <h2>Control</h2>
            </div>
            <div className="column-body">
              <div className="control-block" style={gridSizeStyle(CONTROL_COLS, GRID_ROWS)}>
                <div className="abs-tile bg-magenta input-tile" style={tilePosStyle(0, 0, 2, 1)}>
                  <div className="tile-content input-form-content">
                    <div className="input-title">English</div>
                    <input value={say} onChange={(e) => setSay(e.target.value)} />
                    <button className="tile-arrow" onClick={() => request("/say/" + encodeURIComponent(say))} aria-label="Send English">
                      <img className="tile-arrow-icon" src="/static/metro/arrow.png" alt="" />
                    </button>
                  </div>
                </div>

                <div className="abs-tile bg-lime input-tile" style={tilePosStyle(2, 0, 2, 1)}>
                  <div className="tile-content input-form-content">
                    <div className="input-title">Polish</div>
                    <input value={mow} onChange={(e) => setMow(e.target.value)} />
                    <button className="tile-arrow" onClick={() => request("/mow/" + encodeURIComponent(mow))} aria-label="Send Polish">
                      <img className="tile-arrow-icon" src="/static/metro/arrow.png" alt="" />
                    </button>
                  </div>
                </div>

                <div className="abs-tile bg-cyan input-tile" style={tilePosStyle(4, 0, 2, 1)}>
                  <div className="tile-content input-form-content">
                    <div className="input-title">I&apos;m feeling lucky</div>
                    <input value={guess} onChange={(e) => setGuess(e.target.value)} />
                    <button className="tile-arrow" onClick={() => request("/guess/" + encodeURIComponent(guess))} aria-label="Send guessed language">
                      <img className="tile-arrow-icon" src="/static/metro/arrow.png" alt="" />
                    </button>
                  </div>
                </div>

                <button className="abs-tile bg-orange" style={tilePosStyle(0, 1, 1, 1)} onClick={() => request("/stop")}>
                  <div className="tile-content">Stop</div>
                </button>

                <button
                  className="abs-tile bg-red"
                  style={tilePosStyle(1, 1, 1, 1)}
                  onClick={async () => {
                    await request("/teapot/on");
                    await refreshTeapotStatus();
                  }}
                >
                  <div className="tile-content">Teapot</div>
                </button>

                <div className="abs-tile bg-teal vol-tile" style={tilePosStyle(2, 1, 2, 1)}>
                  <div className="tile-content">
                    <div>Volume: {volume}%</div>
                    <input
                      type="range"
                      min="0"
                      max="100"
                      value={volume}
                      onChange={(e) => setVolume(parseInt(e.target.value, 10))}
                      onMouseUp={() => request("/vol/" + volume)}
                      onTouchEnd={() => request("/vol/" + volume)}
                    />
                  </div>
                </div>

                <label className="abs-tile bg-darkViolet upload-tile" style={tilePosStyle(4, 1, 2, 1)}>
                  <div className="tile-content">
                    <div>Upload MP3</div>
                    <input type="file" accept="audio/mpeg,audio/mp3" onChange={(e) => e.target.files[0] && uploadFile(e.target.files[0])} />
                  </div>
                </label>

                <div className="abs-tile weather-tile" style={tilePosStyle(0, 2, 2, 2)}>
                  <div className="tile-content weather-content">
                    <div className="weather-temp">
                      {weather.ready ? `${weather.temp}\u00b0` : "--\u00b0"}
                    </div>
                    <div className="weather-city">Gdansk</div>
                    <div className="weather-desc">{weatherCodeLabel(weather.code)}</div>
                    <div className="weather-range">
                      Today {weather.ready ? `${weather.max}\u00b0 / ${weather.min}\u00b0` : "-- / --"}
                    </div>
                    <div className="weather-label">Weather</div>
                  </div>
                </div>

                <a
                  className="abs-tile bg-steel mock-link-tile"
                  style={tilePosStyle(2, 2, 1, 1)}
                  href="https://catbox.moe"
                  target="_blank"
                  rel="noreferrer"
                >
                  <div className="tile-content link-tile-content">
                    <img className="link-tile-icon" src="/static/metro/onedrive.png" alt="OneDrive" />
                    <div className="link-tile-label">OneDrive</div>
                  </div>
                </a>

                <a
                  className="abs-tile bg-cyan mock-link-tile"
                  style={tilePosStyle(3, 2, 1, 1)}
                  href="about:about"
                  target="_blank"
                  rel="noreferrer"
                >
                  <div className="tile-content link-tile-content">
                    <img className="link-tile-icon" src="/static/metro/settings.png" alt="Settings" />
                    <div className="link-tile-label">Settings</div>
                  </div>
                </a>

                <a
                  className="abs-tile desktop-link-tile"
                  style={tilePosStyle(4, 2, 1, 1)}
                  href="https://madeupandprobablydoesnotexist.com/taskbar/"
                  target="_blank"
                  rel="noreferrer"
                >
                  <div className="tile-content desktop-link-content">
                    <div className="link-tile-label">Desktop</div>
                  </div>
                </a>

                <a
                  className="abs-tile bg-steel mock-link-tile"
                  style={tilePosStyle(5, 2, 1, 1)}
                  href="/old"
                >
                  <div className="tile-content link-tile-content">
                    <img className="link-tile-icon" src="/static/metro/iexplorer.png" alt="Internet Explorer" />
                    <div className="link-tile-label">Internet Explorer</div>
                  </div>
                </a>

                <div className="abs-tile news-tile" style={tilePosStyle(2, 3, 2, 1)}>
                  <div className="tile-content news-content">
                    <div className="news-kicker">News</div>
                    <div className="news-headline">
                      {teapotKnown && teapotEnabled
                        ? `Breaking: Teapot mode enabled for next ${formatRemaining(teapotRemainingSeconds)}.`
                        : teapotKnown
                          ? "Breaking: Teapot mode disabled."
                          : "Breaking: Teapot mode unknown."}
                    </div>
                  </div>
                </div>

                <a
                  className="abs-tile bg-lime mock-link-tile"
                  style={tilePosStyle(4, 3, 1, 1)}
                  href="https://www.jacktronic.pl/"
                  target="_blank"
                  rel="noreferrer"
                >
                  <div className="tile-content store-tile-content">
                    <img className="store-tile-icon" src="/static/metro/store.png" alt="Store" />
                    <div className="store-tile-title">Store</div>
                  </div>
                </a>

            </div>
          </div>
          </div>

          {categories.map((category, catIdx) => {
            const entries = (files[category] || []).slice().sort();
            const layout = layoutCategory(entries);
            return (
              <div className="column section-block" key={category}>
                <div className="column-head">
                  <h2>{category}</h2>
                </div>
                <div className="column-body">
                  <div className="soundboard-block">
                    <div className="category-grid" style={gridSizeStyle(layout.cols, GRID_ROWS)}>
                      {layout.placed.map((item, idx) => {
                        return (
                          <button
                            className={[
                              "abs-tile",
                              COLORS[(catIdx + idx) % COLORS.length],
                              item.w === 2 ? "tile-w2" : "",
                            ].join(" ")}
                            style={tilePosStyle(item.col, item.row, item.w, 1)}
                            key={category + item.name}
                            onClick={() =>
                              request("/play/" + encodeURIComponent(category) + "/" + encodeURIComponent(item.name))
                            }
                          >
                            <div className="tile-content">
                              <div className="sound-name">{item.name}</div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </section>
      </div>
    </div>
  );
}

export default App;
