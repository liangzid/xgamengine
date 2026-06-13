#|======================================================================
CLI.LISP

1. Command-line entry point for the xgamengine.
2. Usage: sbcl --script bin/cli.lisp [--scenario <name>]
3. (2025-06-13) Phase 6: full interactive CLI with streaming display.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(require :asdf)
(asdf:load-system :xgamengine)
(in-package :xgamengine)

;; ---- ANSI Colors ----

(defparameter *ansi-reset* (format nil "~c[0m" #\Esc))
(defparameter *ansi-cyan* (format nil "~c[36m" #\Esc))
(defparameter *ansi-yellow* (format nil "~c[33m" #\Esc))
(defparameter *ansi-green* (format nil "~c[32m" #\Esc))
(defparameter *ansi-red* (format nil "~c[31m" #\Esc))
(defparameter *ansi-dim* (format nil "~c[2m" #\Esc))
(defparameter *ansi-bold* (format nil "~c[1m" #\Esc))

(defun color (text color-code)
  "Wrap TEXT in ANSI color codes."
  (concatenate 'string color-code text *ansi-reset*))

;; ---- Display Helpers ----

(defun print-banner ()
  "Display the game banner."
  (format t "~a~%" (color "╔══════════════════════════════════════╗" *ansi-cyan*))
  (format t "~a~%" (color "║     修仙模拟器 — xgamengine v0.1.0       ║" *ansi-cyan*))
  (format t "~a~%" (color "║   Xianxia Simulator — AI Text Engine    ║" *ansi-cyan*))
  (format t "~a~%~%" (color "╚══════════════════════════════════════╝" *ansi-cyan*)))

(defun print-narrative (text)
  "Display a narrative block with typewriter-like formatting."
  (format t "~a~%" (color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" *ansi-dim*))
  (format t "~a~%~%" text)
  (format t "~a~%" (color "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" *ansi-dim*))
  (finish-output))

(defun print-status (state)
  "Display the current game status sidebar."
  (format t "~a~%" (color "┌── 修炼状态 ──────────────────────────┐" *ansi-yellow*))
  (format t "~a 境界: ~a~%"
          (color "│" *ansi-yellow*)
          (color (game-state-realm state) *ansi-green*))
  (format t "~a 进度: ~,0f%%  灵力: ~d/~d~%"
          (color "│" *ansi-yellow*)
          (* 100 (game-state-realm-progress state))
          (game-state-qi state)
          (game-state-max-qi state))
  (format t "~a 灵石: ~d  功法: ~d 门~%"
          (color "│" *ansi-yellow*)
          (game-state-spirit-stones state)
          (length (game-state-techniques state)))
  (format t "~a~%" (color "└──────────────────────────────────────┘" *ansi-yellow*))
  (finish-output))

(defun print-suggestions (suggestions)
  "Display suggested actions."
  (format t "~a~%" (color "建议行动:" *ansi-dim*))
  (loop for i from 1
        for s in suggestions
        do (format t "  ~a~a. ~a~a~%"
                   *ansi-dim* i s *ansi-reset*))
  (finish-output))

(defun print-help ()
  "Display command help."
  (format t "~%命令列表:~%")
  (format t "  /status    - 查看当前状态~%")
  (format t "  /save <f>  - 保存游戏到文件~%")
  (format t "  /load <f>  - 从文件加载游戏~%")
  (format t "  /help      - 显示此帮助~%")
  (format t "  /quit      - 退出游戏~%")
  (format t "  直接输入文字即可与修仙世界互动~%~%"))

;; ---- Command Handling ----

(defun handle-command (input)
  "Process a slash command. Returns :quit to exit, :continue otherwise.
INPUT is the raw user input string."
  (let* ((trimmed (string-trim '(#\Space) input))
         (parts (str:split #\Space trimmed :omit-nulls t))
         (cmd (string-downcase (first parts))))
    (cond
      ((string= cmd "/quit")
       (format t "~%道心不灭，仙途再续。告辞！~%")
       :quit)

      ((string= cmd "/help")
       (print-help)
       :continue)

      ((string= cmd "/status")
       (when *engine-state*
         (print-status *engine-state*))
       :continue)

      ((string= cmd "/save")
       (let ((filepath (or (second parts)
                           (format nil "save-~a.json"
                                   (get-universal-time)))))
         (handler-case
             (progn
               (save-game filepath)
               (format t "游戏已保存至: ~a~%" filepath))
           (error (e)
             (format t "保存失败: ~a~%" e))))
       :continue)

      ((string= cmd "/load")
       (let ((filepath (second parts)))
         (if filepath
             (handler-case
                 (progn
                   (load-game filepath)
                   (format t "游戏已从 ~a 加载。~%" filepath)
                   (print-status *engine-state*))
               (error (e)
                 (format t "加载失败: ~a~%" e)))
             (format t "用法: /load <文件名>~%")))
       :continue)

      (t
       (format t "未知命令: ~a。输入 /help 查看命令列表。~%" cmd)
       :continue))))

;; ---- Main Loop ----

(defun repl-loop ()
  "Run the interactive REPL loop."
  (loop
    (format t "~a> ~a" *ansi-green* *ansi-reset*)
    (finish-output)
    (let ((input (read-line *standard-input* nil :eof)))
      (when (or (null input) (eq input :eof))
        (format t "~%再见！~%")
        (return))

      ;; Check for commands
      (when (str:starts-with-p "/" input)
        (let ((result (handle-command input)))
          (when (eq result :quit)
            (return))
          (go :next-loop)))

      ;; Check for empty input
      (when (string= "" (string-trim '(#\Space) input))
        (go :next-loop))

      ;; Process game input
      (format t "~%")
      (handler-case
          (let ((output (process-input input)))
            ;; Display narrative
            (print-narrative (engine-output-narrative output))
            ;; Display state changes
            (when (engine-output-state-changes output)
              (format t "~a~%"
                      (color "【状态变化】" *ansi-yellow*))
              (dolist (change (engine-output-state-changes output))
                (format t "  ~a~%" change)))
            ;; Display suggestions
            (print-suggestions (engine-output-suggestions output))
            ;; Display usage (debug)
            (when (engine-output-usage output)
              (format t "~a~%"
                      (color (format nil "[Tokens: ~d]"
                                     (getf (engine-output-usage output)
                                           :total-tokens 0))
                             *ansi-dim*))))
        (error (e)
          (format t "~a 引擎错误: ~a ~a~%"
                  (color "✗" *ansi-red*) e *ansi-reset*)
          (format t "请重试。如果问题持续，输入 /quit 退出。~%")))
      (terpri))
    :next-loop))

;; ---- Entry Point ----

(defun main ()
  "Main entry point."
  (print-banner)
  (format t "欢迎来到修仙世界！~%")
  (format t "场景: 青云宗 — 山村少年初入仙门~%~%")
  (format t "正在召唤师尊清虚道人...~%~%")

  (handler-case
      (let ((output (start-game :scenario "qingyun" :player-name "无名")))
        (print-narrative (engine-output-narrative output))
        (print-suggestions (engine-output-suggestions output))
        (format t "~%")
        (repl-loop))
    (error (e)
      (format t "~a 启动失败: ~a~a~%"
              (color "✗" *ansi-red*) e *ansi-reset*)
      (format t "请检查: ~%")
      (format t "  1. API Key 是否设置 (环境变量 DEEPSEEK_API_KEY)~%")
      (format t "  2. 网络连接是否正常~%")
      (format t "  3. 模板文件是否在 ../templates/ 目录下~%")
      (sb-ext:exit :code 1))))

;; Run the game
(main)
