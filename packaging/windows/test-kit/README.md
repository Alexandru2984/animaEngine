# Kit de test pentru Windows real

Pentru un Windows 10 sau 11 adevărat (un VM din homelab sau un
calculator), cu installerul unei versiuni. Nu cere drepturi de
administrator.

## Pașii

1. Copiază pe mașina Windows, în același dosar: `Test-AnimaEngine.ps1` și
   installerul (`animaEngine-<versiune>-windows-x86_64-setup.exe`, din
   release-ul de pe GitHub sau din `build/`).
2. Deschide PowerShell în dosarul acela și rulează:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\Test-AnimaEngine.ps1
   ```

3. Scriptul instalează aplicația, o pornește și o măsoară în repaus.
   Apoi te întreabă câteva lucruri pe care doar un om le poate vedea.
   Răspunzi cu `y`, `n` sau `s` (sari peste), iar la un `n` spui ce ai
   văzut. Îți cere și trei acțiuni:
   - pornirea la login, pornită și oprită;
   - raportul de diagnostic, copiat;
   - ieșirea din aplicație, din tray.
4. La final dezinstalează aplicația și verifică că n-a rămas nimic.
   Cu `-KeepInstalled` o lasă instalată.
5. Pe Desktop apare un dosar `anima-test-<dată>`, cu:
   - `report.txt`;
   - jurnalul aplicației;
   - raportul de diagnostic.

   Mi-l trimiți.

## Ce verifică

| Automat | De mână |
|---|---|
| Windows, placa video, ecranele și scalarea, tastaturile | personajele, fără chenar negru sau alb în jur |
| semnătura installerului și a aplicației (SmartScreen) | un clic trece prin suprapunere până dedesubt |
| instalarea: dosarul, Apps & features, meniul Start | iconița din tray și meniul ei |
| pornirea, CPU și memoria în repaus | rotița deschide modul de editare cu bun-venitul |
| erorile din jurnal | un personaj tras cade înapoi |
| pornirea la login: intrarea `Run` scrisă, apoi ștearsă | Ctrl+Shift+A cu Notepad în față |
| raportul de diagnostic ajunge în clipboard | claritatea la scalare peste 100% |
| dezinstalarea nu lasă nimic în urmă | al doilea ecran, dacă există |

Setările și imaginile din `%APPDATA%\animaEngine` rămân după
dezinstalare, intenționat. Ca să începi de la zero, le ștergi de mână.
