// Avisa o Super Cola sobre o app em foco. Painéis e menus não contam: o alvo da colagem
// continua sendo a última janela comum.
function avisar(janela) {
    if (!janela || janela.resourceClass === "supercola") {
        return;
    }
    if (!janela.normalWindow && !janela.dialog) {
        return;
    }
    callDBus("dev.supercola.Daemon", "/dev/supercola/Bridge", "dev.supercola.Bridge",
             "FocusChanged", String(janela.resourceClass));
}

workspace.windowActivated.connect(avisar);
avisar(workspace.activeWindow);
