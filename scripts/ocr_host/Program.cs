// ClipNest OcrHost: thin CLI shim that runs WeChatOcr (ZGGSONG) and emits
// the recognized text on stdout. Used by the Tauri Rust side as a child
// process; the contract is:
//
//   $ wcocr <image_path>            -> stdout: recognized text
//   $ wcocr <image_path> --json     -> stdout: { "text": "..." } JSON
//
// Exit codes:
//   0   success
//   2   invalid usage (no arg)
//   3   image path missing
//   4   WeChatOcr threw (engine error)
//   5   WeChatOcr dependency missing (mmmojo / wco_data)
using System;
using System.Linq;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using WeChatOcr;

namespace ClipNest.OcrHost;

internal static class Program
{
    private static int Main(string[] args)
    {
        Console.OutputEncoding = Encoding.UTF8;
        Console.InputEncoding = Encoding.UTF8;

        if (args.Length < 1)
        {
            Console.Error.WriteLine("usage: wcocr <image_path> [--json]");
            return 2;
        }
        var imagePath = args[0];
        var wantJson = args.Length > 1 && args[1] == "--json";
        if (!File.Exists(imagePath))
        {
            Console.Error.WriteLine($"image not found: {imagePath}");
            return 3;
        }
        try
        {
            var imageData = File.ReadAllBytes(imagePath);
            var tcs = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
            using var ocr = new ImageOcr();
            ocr.Run(imageData, (path, result) =>
            {
                try
                {
                    if (result?.OcrResult?.SingleResult is { } list)
                    {
                        var sb = new StringBuilder();
                        foreach (var item in list)
                        {
                            if (string.IsNullOrEmpty(item?.SingleStrUtf8)) continue;
                            sb.AppendLine(RepairMojibake(item.SingleStrUtf8));
                        }
                        tcs.TrySetResult(sb.ToString());
                    }
                    else
                    {
                        tcs.TrySetResult(string.Empty);
                    }
                }
                catch (Exception ex)
                {
                    tcs.TrySetException(ex);
                }
            }, ImageType.Png);
            // Safety: cap to 10s in case the OCR engine hangs.
            if (!tcs.Task.Wait(TimeSpan.FromSeconds(10)))
            {
                Console.Error.WriteLine("ocr timed out");
                return 4;
            }
            var text = tcs.Task.Result.TrimEnd('\r', '\n');
            if (wantJson)
            {
                Console.WriteLine("{\"text\":" + JsonEscape(text) + "}");
            }
            else
            {
                Console.WriteLine(text);
            }
            return 0;
        }
        catch (DllNotFoundException ex)
        {
            Console.Error.WriteLine("wechat_ocr dependency missing: " + ex.Message);
            return 5;
        }
        catch (FileNotFoundException ex)
        {
            Console.Error.WriteLine("wechat_ocr data missing: " + ex.Message);
            return 5;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("wechat_ocr error: " + ex.Message);
            return 4;
        }
    }

    private static string RepairMojibake(string raw)
    {
        var candidates = new[]
        {
            raw,
            ReDecode(raw, Encoding.Latin1, Encoding.UTF8),
        }
        .Where(s => !string.IsNullOrWhiteSpace(s))
        .Distinct()
        .OrderByDescending(ScoreText)
        .ToArray();

        return candidates.FirstOrDefault() ?? raw;
    }

    private static string ReDecode(string raw, Encoding from, Encoding to)
    {
        try
        {
            return to.GetString(from.GetBytes(raw));
        }
        catch
        {
            return raw;
        }
    }

    private static int ScoreText(string s)
    {
        var cjk = s.Count(ch => ch >= 0x4E00 && ch <= 0x9FFF);
        var ascii = s.Count(ch => ch >= 0x20 && ch <= 0x7E);
        var controls = s.Count(char.IsControl);
        var mojibake = s.Count(ch => "ÃÂÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ�".Contains(ch));
        return cjk * 12 + ascii - controls * 8 - mojibake * 4;
    }

    private static string JsonEscape(string s)
    {
        var sb = new StringBuilder(s.Length + 2);
        sb.Append('"');
        foreach (var c in s)
        {
            switch (c)
            {
                case '"': sb.Append("\\\""); break;
                case '\\': sb.Append("\\\\"); break;
                case '\b': sb.Append("\\b"); break;
                case '\f': sb.Append("\\f"); break;
                case '\n': sb.Append("\\n"); break;
                case '\r': sb.Append("\\r"); break;
                case '\t': sb.Append("\\t"); break;
                default:
                    if (c < 0x20) sb.AppendFormat("\\u{0:x4}", (int)c);
                    else sb.Append(c);
                    break;
            }
        }
        sb.Append('"');
        return sb.ToString();
    }
}
