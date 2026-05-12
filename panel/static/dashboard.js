// dashboard.js
// Oczekuje zmiennych wstrzykniętych przez serwer (inline):
//   window.INIT_DATA  – tablica { ts, temp, humidity }
//   window.MAX_POINTS – liczba punktów na wykresie
//   window.SSE_URL    – URL endpointu SSE

// ─── Stan globalny ────────────────────────────────────────────────────────────
let updateCount = 0;
let prevTemp = null;
let prevHumid = null;

// Tryb widoku: { mode: 'samples'|'time', n: number }
// samples → MAX_POINTS = n, oś X auto
// time    → oś X stała [now-n, now], brak przycinania danych
function parseViewValue(value) {
    const [mode, n] = (value ?? 'samples:60').split(':');
    return { mode, n: parseInt(n) };
}

let currentView = parseViewValue(localStorage.getItem('km-view') ?? 'samples:60');

function getMaxPoints() {
    return currentView.mode === 'samples' ? currentView.n : Infinity;
}

// Alias dla kompatybilności z miejscami które używają MAX_POINTS
Object.defineProperty(window, 'MAX_POINTS', {
    get: () => getMaxPoints(),
    configurable: true,
});

// ─── Dane startowe ────────────────────────────────────────────────────────────
const initialData = window.INIT_DATA ?? [];

let tsArr = initialData.map(d => d.ts);
let tempArr = initialData.map(d => d.temp);
let humidArr = initialData.map(d => d.humidity);

if (tsArr.length > getMaxPoints()) {
    const start = tsArr.length - getMaxPoints();
    tsArr = tsArr.slice(start);
    tempArr = tempArr.slice(start);
    humidArr = humidArr.slice(start);
}

const plotData = [tsArr, tempArr, humidArr];

// ─── Kolory per motyw ─────────────────────────────────────────────────────────
const CHART_COLORS = {
    light: {
        bg: 'transparent',
        axisStroke: '#6e6e73',
        axisTick: '#c7c7cc',
        axisGrid: '#d1d1d6',
        tempStroke: '#ff3b30',
        tempFill: 'rgba(255,59,48,0.06)',
        humidStroke: '#007aff',
        humidFill: 'rgba(0,122,255,0.06)',
    },
    dark: {
        bg: 'transparent',
        axisStroke: '#98989d',
        axisTick: '#48484a',
        axisGrid: '#3a3a3c',
        tempStroke: '#ff453a',
        tempFill: 'rgba(255,69,58,0.12)',
        humidStroke: '#0a84ff',
        humidFill: 'rgba(10,132,255,0.12)',
    },
};

// ─── Typ wykresu ──────────────────────────────────────────────────────────────
let currentChartType = localStorage.getItem('km-chart-type') ?? 'line';

// Buduje paths dla danego typu — null = domyślna linia uPlot
function buildPaths(type) {
    switch (type) {
        case 'bars': return uPlot.paths.bars({ size: [0.6, 100] });
        case 'stepped': return uPlot.paths.stepped({ align: 1 });
        case 'points': return () => null;  // brak linii, tylko punkty
        default: return null;        // line / area
    }
}

// Czy dany typ wymaga wypełnienia pod krzywą
function useFill(type) {
    return type === 'area' || type === 'bars';
}

// ─── Pomocnicze ───────────────────────────────────────────────────────────────
function getChartWidth() {
    const el = document.getElementById('chart-container');
    return el ? el.clientWidth : 800;
}

function currentTheme() {
    return document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
}

// ─── Budowanie opcji uPlot ────────────────────────────────────────────────────
function buildOpts(theme) {
    const c = CHART_COLORS[theme];
    const font = '11px SF Mono, ui-monospace, Menlo, monospace';
    const paths = buildPaths(currentChartType);
    const fill = useFill(currentChartType);
    const showPoints = currentChartType === 'points';
    const lineWidth = currentChartType === 'bars' ? 1 : 2;

    return {
        width: getChartWidth(),
        height: 280,
        padding: [16, 0, 0, 0],
        background: c.bg,
        cursor: { points: { size: 6 } },
        legend: { show: false },
        axes: [
            // X – czas
            {
                stroke: c.axisStroke,
                ticks: { stroke: c.axisTick, width: 1 },
                grid: { stroke: c.axisGrid, width: 1 },
                font,
                labelFont: font,
                labelGap: 8,
                values: (self, ticks) => ticks.map(t => {
                    const d = new Date(t * 1000);
                    return [d.getHours(), d.getMinutes(), d.getSeconds()]
                        .map(n => String(n).padStart(2, '0')).join(':');
                }),
            },
            // Y lewa – temperatura
            {
                scale: 'temp',
                label: '°C',
                labelSize: 20,
                font,
                labelFont: font,
                stroke: c.tempStroke,
                ticks: { stroke: c.axisTick, width: 1 },
                grid: { stroke: c.axisGrid, width: 1 },
                values: (self, ticks) => ticks.map(v => v.toFixed(1)),
                side: 3,
                size: 52,
            },
            // Y prawa – wilgotność
            {
                scale: 'humidity',
                label: '%',
                labelSize: 20,
                font,
                labelFont: font,
                stroke: c.humidStroke,
                ticks: { stroke: c.axisTick, width: 1 },
                grid: { show: false },
                values: (self, ticks) => ticks.map(v => v.toFixed(1)),
                side: 1,
                size: 52,
            },
        ],
        scales: {
            x: {
                time: true,
                // Tryb time: stałe okno przesuwające się z czasem
                // Tryb samples: auto (dopasowane do danych)
                range: currentView.mode === 'time'
                    ? () => {
                        const now = Date.now() / 1000;
                        return [now - currentView.n, now];
                    }
                    : null,
            },
            temp: { auto: true, range: (_, min, max) => [Math.floor(min - 1), Math.ceil(max + 1)] },
            humidity: { auto: true, range: (_, min, max) => [Math.floor(min - 2), Math.ceil(max + 2)] },
        },
        series: [
            {},
            {
                label: 'Temperatura',
                scale: 'temp',
                stroke: c.tempStroke,
                width: lineWidth,
                fill: fill ? c.tempFill : undefined,
                paths,
                spanGaps: true,
                points: { show: showPoints, size: 5, stroke: c.tempStroke, fill: c.tempFill },
            },
            {
                label: 'Wilgotność',
                scale: 'humidity',
                stroke: c.humidStroke,
                width: lineWidth,
                fill: fill ? c.humidFill : undefined,
                paths,
                spanGaps: true,
                points: { show: showPoints, size: 5, stroke: c.humidStroke, fill: c.humidFill },
            },
        ],
    };
}

// ─── uPlot — tworzenie i reinicjalizacja ──────────────────────────────────────
let uplot = null;

function createChart(theme) {
    const container = document.getElementById('chart-container');
    if (!container) return;

    if (uplot) {
        uplot.destroy();
        uplot = null;
    }

    uplot = new uPlot(buildOpts(theme), plotData, container);
}

// Pierwsze renderowanie
createChart(currentTheme());

new ResizeObserver(() => {
    if (uplot) uplot.setSize({ width: getChartWidth(), height: 280 });
}).observe(document.getElementById('chart-container'));

// ─── Obsługa nowego punktu pomiarowego ───────────────────────────────────────
function onSensorData({ ts, temp, humidity }) {
    const dispTemp = document.getElementById('disp-temp');
    const dispHumid = document.getElementById('disp-humid');
    const trendTemp = document.getElementById('trend-temp');
    const trendHumid = document.getElementById('trend-humid');

    function flashUpdate(el) {
        el.parentElement.classList.remove('value-updated');
        void el.parentElement.offsetWidth;
        el.parentElement.classList.add('value-updated');
    }

    if (dispTemp) { dispTemp.textContent = temp.toFixed(1); flashUpdate(dispTemp); }
    if (dispHumid) { dispHumid.textContent = humidity.toFixed(1); flashUpdate(dispHumid); }

    if (trendTemp && prevTemp !== null) {
        const d = temp - prevTemp;
        trendTemp.textContent =
            `${d > 0.05 ? '↑' : d < -0.05 ? '↓' : '→'} ${d >= 0 ? '+' : ''}${d.toFixed(1)}°C od ostatniego pomiaru`;
    }
    if (trendHumid && prevHumid !== null) {
        const d = humidity - prevHumid;
        trendHumid.textContent =
            `${d > 0.1 ? '↑' : d < -0.1 ? '↓' : '→'} ${d >= 0 ? '+' : ''}${d.toFixed(1)}% od ostatniego pomiaru`;
    }
    prevTemp = temp;
    prevHumid = humidity;

    plotData[0].push(ts);
    plotData[1].push(temp);
    plotData[2].push(humidity);
    // W trybie samples przycinamy do n; w trybie time nie przycinamy
    if (currentView.mode === 'samples') {
        while (plotData[0].length > currentView.n) {
            plotData[0].shift();
            plotData[1].shift();
            plotData[2].shift();
        }
    } else {
        // Usuń punkty starsze niż okno czasowe
        const cutoff = Date.now() / 1000 - currentView.n;
        while (plotData[0].length > 0 && plotData[0][0] < cutoff) {
            plotData[0].shift();
            plotData[1].shift();
            plotData[2].shift();
        }
    }

    if (uplot) uplot.setData(plotData);

    updateCount++;
    const counter = document.getElementById('update-count');
    if (counter) counter.textContent = updateCount;
}

// ─── SSE – połączenie z auto-reconnect, dwufazowy strumień ──────────────────
// Faza 1: zdarzenia "history" (replay z bazy) + "history_end"
// Faza 2: zdarzenia "sensor" (live)

let historyPhase = false;  // true podczas ładowania historii z bazy

// Buduje URL SSE — tryb time dodaje ?from= dla historii z bazy
function buildSseUrl() {
    const base = window.SSE_URL ?? '/sse';
    if (currentView.mode !== 'time') return base;
    const from = (Date.now() / 1000 - currentView.n).toFixed(3);
    return `${base}?from=${from}`;
}

let _currentEs = null;

function connectSSE() {
    // Zamknij poprzednie połączenie jeśli istnieje
    if (_currentEs) {
        _currentEs.close();
        _currentEs = null;
    }
    const url = buildSseUrl();
    const es = new EventSource(url);

    // ── Faza 1: punkt historyczny z bazy ─────────────────────────────────────
    es.addEventListener('history', (e) => {
        historyPhase = true;
        try {
            const { ts, temp, humidity } = JSON.parse(e.data);
            plotData[0].push(ts);
            plotData[1].push(temp);
            plotData[2].push(humidity);
            // Nie przycinamy podczas fazy historii — wyświetlamy cały zakres
        } catch (err) {
            console.error('SSE history parse error:', err);
        }
    });

    // ── Koniec fazy historii ──────────────────────────────────────────────────
    es.addEventListener('history_end', (e) => {
        historyPhase = false;
        try {
            const { count } = JSON.parse(e.data);
            console.info(`[SSE] Historia załadowana: ${count} punktów`);
        } catch (_) { }
        // Renderuj wykres po załadowaniu całej historii
        if (uplot) uplot.setData(plotData);
        showLiveIndicator();
    });

    // ── Faza 2: live punkt ────────────────────────────────────────────────────
    es.addEventListener('sensor', (e) => {
        try {
            onSensorData(JSON.parse(e.data));
        } catch (err) {
            console.error('SSE sensor parse error:', err);
        }
    });

    es.onerror = () => {
        es.close();
        setTimeout(connectSSE, 2000);
    };

    _currentEs = es;
}

// Pierwsze połączenie
connectSSE();

// Pokazuje wskaźnik live po zakończeniu fazy historii
function showLiveIndicator() {
    const pill = document.getElementById('live-indicator');
    if (pill) pill.style.display = 'flex';
}

// ─── Motyw — ustawienie i inicjalizacja ──────────────────────────────────────
function setTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('km-theme', theme);
    updateThemeButtons(theme);
    createChart(theme);
}

function updateThemeButtons(theme) {
    const btnLight = document.getElementById('btn-light');
    const btnDark = document.getElementById('btn-dark');
    if (btnLight) btnLight.classList.toggle('active', theme === 'light');
    if (btnDark) btnDark.classList.toggle('active', theme === 'dark');
}

(function initTheme() {
    const saved = localStorage.getItem('km-theme');
    const system = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    const theme = saved ?? system;

    document.documentElement.setAttribute('data-theme', theme);
    updateThemeButtons(theme);

    if (theme === 'dark') createChart('dark');
})();

// ─── Typ wykresu — zmiana ─────────────────────────────────────────────────────
function onChartTypeChange(type) {
    currentChartType = type;
    localStorage.setItem('km-chart-type', type);
    createChart(currentTheme());
}

(function initChartType() {
    const select = document.getElementById('chart-type-select');
    if (select && currentChartType !== 'line') {
        select.value = currentChartType;
        createChart(currentTheme());
    }
})();

// ─── Widok — zmiana trybu (próbki / czas) ─────────────────────────────────────
function onViewChange(value) {
    currentView = parseViewValue(value);
    localStorage.setItem('km-view', value);

    // Wyczyść dane
    plotData[0].length = 0;
    plotData[1].length = 0;
    plotData[2].length = 0;

    // Ukryj wskaźnik live — będzie pokazany po history_end (tryb time)
    // lub od razu w trybie samples
    const pill = document.getElementById('live-indicator');

    if (currentView.mode === 'samples') {
        // Tryb próbek — tylko live, bez historii z bazy
        if (pill) pill.style.display = 'none';
        createChart(currentTheme());
        connectSSE();
    } else {
        // Tryb czasu — ładuj historię z bazy + live
        if (pill) pill.style.display = 'none';
        createChart(currentTheme());
        connectSSE();
    }
}

// Odśwież zakres X w trybie time co sekundę
// (oś przesuwa się płynnie wraz z upływem czasu)
/* setInterval(() => {
    if (currentView.mode === 'time' && uplot) {
        // Usuń dane starsze niż okno
        const cutoff = Date.now() / 1000 - currentView.n;
        while (plotData[0].length > 0 && plotData[0][0] < cutoff) {
            plotData[0].shift();
            plotData[1].shift();
            plotData[2].shift();
        }
        uplot.setData(plotData);
    }
}, 1000); */

(function initView() {
    const saved = localStorage.getItem('km-view') ?? 'samples:60';
    const select = document.getElementById('view-select');
    if (select) select.value = saved;
    currentView = parseViewValue(saved);

    // Jeśli tryb time — pierwsze połączenie SSE z ?from=
    // (connectSSE jest wywoływany po initTheme przez istniejący kod)
})();
