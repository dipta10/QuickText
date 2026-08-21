import "./styles.css";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root was not found");
}

app.innerHTML = `
  <button class="record-button" type="button" aria-label="Start recording">
    Record
  </button>
`;
