import init, { WebHandle } from "./generated/easy_nats";
import "./style.css";

const canvas = document.querySelector<HTMLCanvasElement>("#easy-nats-canvas");
const status = document.querySelector<HTMLElement>("#demo-status");

if (!canvas || !status) {
  throw new Error("Interactive demo mount point is missing");
}

let demoStarted = false;

const startDemo = async () => {
  if (demoStarted) {
    return;
  }
  demoStarted = true;

  try {
    await init();
    const demo = new WebHandle();
    (
      window as Window & {
        easyNatsDemo?: WebHandle;
      }
    ).easyNatsDemo = demo;
    await demo.start(canvas);
    status.remove();
  } catch (error) {
    status.textContent =
      "The interactive demo could not start. / 交互体验暂时无法启动。";
    console.error(error);
  }
};

const interactiveViewport = window.matchMedia("(min-width: 721px)");
if (interactiveViewport.matches) {
  await startDemo();
} else {
  interactiveViewport.addEventListener("change", (event) => {
    if (event.matches) {
      void startDemo();
    }
  });
}
