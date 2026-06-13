#|======================================================================
ENGINE.LISP

1. Main game loop — orchestrates state, memory, prompt building, and LLM calls.
2. Calling chain:
     CLI → engine:start-game / engine:process-input
        → prompt/builder:build-messages
        → llm/client:call-chat-completion
        → state:apply-state-change
        → memory:append-turn!
     CLI ← engine:engine-output
3. (2025-06-13) Phase 5: full game loop with API integration.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- Engine State ----

(defvar *engine-state* nil
  "The current GAME-STATE instance for the running game session.")

(defvar *engine-window* nil
  "The current CONVERSATION-WINDOW for the running game session.")

(defvar *engine-config* nil
  "Engine configuration plist (:scenario :player-name :npc ...).")

;; ---- Engine Output ----

(defstruct engine-output
  "Output from a single engine step."
  (narrative "" :type string)           ; AI-generated narrative text
  (state-changes nil :type list)        ; list of change plists applied
  (suggestions nil :type list)          ; 3 suggested next actions
  (round 0 :type integer)               ; current round number
  (usage nil :type list))               ; token usage plist from API

;; ---- Game Initialization ----

(defun start-game (&key (scenario "qingyun") (player-name "无名"))
  "Start a new game session.
Returns an ENGINE-OUTPUT with the opening narrative."
  ;; Initialize state
  (setf *engine-state* (make-initial-state :scenario scenario
                                           :player-name player-name))
  (setf *engine-window* (make-conversation-window))
  (setf *engine-config* (list :scenario scenario
                              :player-name player-name
                              :npc "qingxu"))

  ;; Build the opening system prompt + initial user request
  (let* ((opening-input (build-opening-input scenario player-name))
         (messages (build-messages *engine-state*
                                   *engine-window*
                                   opening-input))
         (response (call-chat-completion messages
                                         :temperature 0.9
                                         :max-tokens 512))
         (narrative (extract-assistant-content response))
         (usage (extract-usage response)))

    ;; Record the opening turn in the window
    (append-turn! *engine-window*
                  opening-input
                  (list :role "assistant" :content narrative))

    ;; Generate suggestions
    (let ((suggestions (generate-suggestions *engine-state*)))
      (make-engine-output
       :narrative narrative
       :state-changes nil
       :suggestions suggestions
       :round 0
       :usage usage))))

(defun build-opening-input (scenario player-name)
  "Build the opening player input string based on the scenario."
  (declare (ignore player-name))
  (cond
    ((string= scenario "qingyun")
     "你睁开双眼，发现自己正盘坐在一处陌生的石洞中。灵气在四周流淌，你隐约记得自己刚刚拜入青云宗。请描述此刻的场景，并引入我的师尊清虚道人。")
    (t
     (format nil "开始修仙之旅，场景：~a" scenario))))

;; ---- Core Game Step ----

(defun process-input (user-input)
  "Process a player input and return an ENGINE-OUTPUT.
USER-INPUT is the player's text input.
This is the core game loop function."
  (unless *engine-state*
    (error "No active game session. Call START-GAME first."))

  ;; Advance round
  (incf (game-state-round *engine-state*))

  ;; Build messages and call API
  (let* ((messages (build-messages *engine-state*
                                   *engine-window*
                                   user-input))
         (response (call-chat-completion messages
                                         :temperature 0.9
                                         :max-tokens 512))
         (narrative (extract-assistant-content response))
         (usage (extract-usage response)))

    (unless narrative
      (error "No narrative content in API response. Full response: ~s" response))

    ;; Extract state changes from the narrative
    (let* ((assistant-msg (list :role "assistant" :content narrative))
           (changes (extract-state-changes assistant-msg *engine-state*)))

      ;; Apply state changes
      (when changes
        (apply-state-change *engine-state* changes))

      ;; Record the turn
      (append-turn! *engine-window* user-input assistant-msg)

      ;; Update last narrative in state
      (setf (game-state-last-narrative *engine-state*) narrative)

      ;; Generate suggestions
      (let ((suggestions (generate-suggestions *engine-state*)))
        (make-engine-output
         :narrative narrative
         :state-changes (when changes (list changes))
         :suggestions suggestions
         :round (game-state-round *engine-state*)
         :usage usage)))))

;; ---- Save / Load ----

(defun save-game (filepath)
  "Save the current game session to FILEPATH.
Returns T on success."
  (unless *engine-state*
    (error "No active game session to save."))
  (let ((json-str (serialize-state *engine-state*)))
    (with-open-file (out filepath :direction :output
                              :if-exists :supersede
                              :if-does-not-exist :create)
      (write-string json-str out))
    t))

(defun load-game (filepath)
  "Load a game session from FILEPATH.
Replaces the current *ENGINE-STATE* and *ENGINE-WINDOW*.
Returns the loaded GAME-STATE."
  (let ((json-str (uiop:read-file-string filepath)))
    (setf *engine-state* (deserialize-state json-str))
    ;; Create a fresh conversation window (history is not persisted in MVP)
    (setf *engine-window* (make-conversation-window))
    *engine-state*))

;; ---- Status Display ----

(defun display-status ()
  "Return a formatted status string for the current game state."
  (unless *engine-state*
    (return-from display-status "No active game."))
  (state-to-narrative *engine-state*))
