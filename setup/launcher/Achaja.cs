// Achaja.exe - okno programu Achaja (bez konsoli). Uruchamia aplikację z lokalnego .venv i czeka na jej koniec,
// więc w Menedżerze zadań i na pasku Achaja jest jednym programem z własną ikoną.
//   Achaja.exe                 okno Achai (argumenty idą dalej: --continue, --minimized)
//   Achaja.exe --settings      kreator ustawień
// Budowanie: setup\build-installer.ps1 albo install.ps1 (kompilator C# z .NET Framework, jest w każdym Windows).
using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Windows.Forms;

[assembly: AssemblyTitle("Achaja")]
[assembly: AssemblyProduct("Achaja")]
[assembly: AssemblyDescription("Achaja - voice assistant for Claude Code")]
[assembly: AssemblyCompany("Banzamel")]
[assembly: AssemblyVersion("__VERSION__")]
[assembly: AssemblyFileVersion("__VERSION__")]

static class Launcher
{
    static string Quote(string s)
    {
        return "\"" + s.Replace("\"", "\\\"") + "\"";
    }

    [STAThread]
    static int Main(string[] args)
    {
        string root = AppDomain.CurrentDomain.BaseDirectory;
        string pythonw = Path.Combine(root, @".venv\Scripts\pythonw.exe");
        // okno Achai to pakiet src\achaja (python -m achaja.ui.app); kreator ustawien - skrypt setup\configure.py
        string[] target = { "-m", "achaja.ui.app" };
        string check = Path.Combine(root, @"src\achaja\ui\app.py");
        if (args.Length > 0 && args[0] == "--settings")
        {
            check = Path.Combine(root, @"setup\configure.py");
            target = new[] { check };
            args = args.Skip(1).ToArray();
        }
        if (!File.Exists(pythonw) || !File.Exists(check))
        {
            MessageBox.Show("Nie znaleziono środowiska Achai (.venv). Uruchom instalator ponownie.\n\n" +
                            "Achaja's environment (.venv) was not found. Run the installer again.",
                            "Achaja", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
        var info = new ProcessStartInfo(pythonw, string.Join(" ", target.Concat(args).Select(Quote)))
        {
            WorkingDirectory = root,
            UseShellExecute = false,
        };
        info.EnvironmentVariables["PYTHONPATH"] = Path.Combine(root, "src");  // tez bez achaja.pth w .venv
        using (var proc = Process.Start(info))
        {
            proc.WaitForExit();
            return proc.ExitCode;
        }
    }
}
