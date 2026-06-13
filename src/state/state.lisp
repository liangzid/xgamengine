#|======================================================================
STATE.LISP

1. Game state management — struct, serialization, deserialization, state-to-narrative.
2. Called by: engine.lisp → state.lisp ↔ JSON ↔ disk (save/load).
3. (2025-06-13) Phase 3: full state struct with JSON round-trip.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- Game State Structure ----

(defstruct game-state
  "Represents the complete game state for a cultivation simulation."
  (id "" :type string)
  (scenario "qingyun" :type string)
  (round 0 :type integer)

  ;; Cultivation
  (realm "练气期初期" :type string)
  (realm-progress 0.0 :type single-float)   ; 0.0 ~ 1.0 (fraction toward next breakthrough)
  (qi 100 :type integer)
  (max-qi 100 :type integer)

  ;; Techniques & Items
  (techniques nil :type list)    ; list of strings (technique names)
  (treasures nil :type list)     ; list of strings (item names)
  (spirit-stones 0 :type integer)

  ;; Social
  (sect "青云宗" :type string)
  (relationships nil :type list) ; list of plists: (:name \"...\" :affinity 0)

  ;; Narrative tracking
  (flags nil :type list)         ; list of string flags, e.g. ("met-master" "first-kill")
  (recent-events nil :type list) ; last ~5 event descriptions
  (last-narrative "" :type string))

;; ---- Initial State ----

(defun make-initial-state (&key (scenario "qingyun") (player-name "无名"))
  "Create a fresh game state for a new game.
SCENARIO selects the starting scenario.
PLAYER-NAME sets the player character's name."
  (make-game-state
   :id (format nil "~a-~a" scenario (get-universal-time))
   :scenario scenario
   :round 0
   :realm "练气期初期"
   :realm-progress 0.0
   :qi 100
   :max-qi 100
   :techniques '("青云吐纳术")
   :treasures '("凡铁剑")
   :spirit-stones 50
   :sect "青云宗"
   :relationships (list (list :name "清虚道人" :role "师尊" :affinity 20))
   :flags (list "game-started" (format nil "player-name-~a" player-name))))

;; ---- Realm Helpers ----

(defparameter *realm-order*
  '("练气期初期" "练气期中期" "练气期后期" "练气期圆满"
    "筑基期初期" "筑基期中期" "筑基期后期" "筑基期圆满"
    "金丹期初期" "金丹期中期" "金丹期后期" "金丹期圆满"
    "元婴期初期" "元婴期中期" "元婴期后期" "元婴期圆满"
    "化神期初期" "化神期中期" "化神期后期" "化神期圆满")
  "Ordered list of cultivation realms.")

(defun realm-index (realm-name)
  "Return the numeric index of REALM-NAME in the realm order, or NIL if not found."
  (position realm-name *realm-order* :test #'string=))

(defun advance-realm (current-realm)
  "Return the next realm after CURRENT-REALM, or NIL if already at max."
  (let ((idx (realm-index current-realm)))
    (when (and idx (< idx (1- (length *realm-order*))))
      (nth (1+ idx) *realm-order*))))

(defun is-breakthrough-p (current-realm)
  "Return T if advancing from CURRENT-REALM crosses a major realm boundary."
  (let ((idx (realm-index current-realm)))
    (when idx
      ;; breakthrough at indices 3→4, 7→8, 11→12, 15→16, 19→20
      (member idx '(3 7 11 15 19)))))

;; ---- State-to-Narrative ----

(defun state-to-narrative (state)
  "Convert the current GAME-STATE into a natural-language description
suitable for injecting into the LLM system prompt.
Returns a string in Chinese."
  (with-output-to-string (s)
    (format s "【角色状态】~%")
    (format s "修炼境界: ~a~%" (game-state-realm state))
    (format s "修炼进度: ~,0f%%~%" (* 100 (game-state-realm-progress state)))
    (format s "灵力: ~d/~d~%" (game-state-qi state) (game-state-max-qi state))
    (format s "功法: ~{~a~^、~}~%" (or (game-state-techniques state) '("无")))
    (format s "灵石: ~d~%" (game-state-spirit-stones state))
    (format s "宗门: ~a~%" (game-state-sect state))
    (when (game-state-treasures state)
      (format s "物品: ~{~a~^、~}~%" (game-state-treasures state)))
    (when (game-state-relationships state)
      (format s "人物关系: ~%")
      (dolist (rel (game-state-relationships state))
        (format s "  - ~a (~a) 好感: ~d~%"
                (getf rel :name)
                (getf rel :role)
                (getf rel :affinity))))
    (when (game-state-recent-events state)
      (format s "近期事件: ~%")
      (dolist (event (game-state-recent-events state))
        (format s "  - ~a~%" event)))))

;; ---- State Mutations ----

(defun apply-state-change (state change-plist)
  "Apply a state change described by CHANGE-PLIST to STATE (mutates STATE).
CHANGE-PLIST keys: :realm-progress, :qi-delta, :add-technique, :add-treasure,
:spirit-stones-delta, :add-flag, :add-event.
Returns the modified STATE."
  (let ((rp (getf change-plist :realm-progress))
        (qd (getf change-plist :qi-delta))
        (at (getf change-plist :add-technique))
        (atre (getf change-plist :add-treasure))
        (ssd (getf change-plist :spirit-stones-delta))
        (af (getf change-plist :add-flag))
        (ae (getf change-plist :add-event)))
    (when rp
      (setf (game-state-realm-progress state)
            (min 1.0 (max 0.0 (+ (game-state-realm-progress state) rp))))
      ;; Check for breakthrough
      (when (>= (game-state-realm-progress state) 1.0)
        (let ((next (advance-realm (game-state-realm state))))
          (if next
              (progn
                (setf (game-state-realm state) next)
                (setf (game-state-realm-progress state) 0.0)
                (push (format nil "境界突破: ~a → ~a"
                              (game-state-realm state) next)
                      (game-state-recent-events state)))
              (setf (game-state-realm-progress state) 1.0)))))
    (when qd
      (setf (game-state-qi state)
            (max 0 (min (game-state-max-qi state)
                        (+ (game-state-qi state) qd)))))
    (when at
      (push at (game-state-techniques state)))
    (when atre
      (push atre (game-state-treasures state)))
    (when ssd
      (setf (game-state-spirit-stones state)
            (max 0 (+ (game-state-spirit-stones state) ssd))))
    (when af
      (push af (game-state-flags state)))
    (when ae
      (push ae (game-state-recent-events state))
      ;; Keep only last 10 events
      (when (> (length (game-state-recent-events state)) 10)
        (setf (game-state-recent-events state)
              (subseq (game-state-recent-events state) 0 10)))))
  state)

;; ---- Serialization ----

(defun state-to-plist (state)
  "Convert a GAME-STATE struct to a plist suitable for JSON encoding."
  (list :id (game-state-id state)
        :scenario (game-state-scenario state)
        :round (game-state-round state)
        :realm (game-state-realm state)
        :realm-progress (game-state-realm-progress state)
        :qi (game-state-qi state)
        :max-qi (game-state-max-qi state)
        :techniques (game-state-techniques state)
        :treasures (game-state-treasures state)
        :spirit-stones (game-state-spirit-stones state)
        :sect (game-state-sect state)
        :relationships (game-state-relationships state)
        :flags (game-state-flags state)
        :recent-events (game-state-recent-events state)
        :last-narrative (game-state-last-narrative state)))

(defun plist-to-state (plist)
  "Convert a plist back to a GAME-STATE struct."
  (make-game-state
   :id (getf plist :id "")
   :scenario (getf plist :scenario "qingyun")
   :round (getf plist :round 0)
   :realm (getf plist :realm "练气期初期")
   :realm-progress (float (getf plist :realm-progress 0.0) 0.0)
   :qi (getf plist :qi 100)
   :max-qi (getf plist :max-qi 100)
   :techniques (getf plist :techniques nil)
   :treasures (getf plist :treasures nil)
   :spirit-stones (getf plist :spirit-stones 0)
   :sect (getf plist :sect "青云宗")
   :relationships (getf plist :relationships nil)
   :flags (getf plist :flags nil)
   :recent-events (getf plist :recent-events nil)
   :last-narrative (getf plist :last-narrative "")))

(defun serialize-state (state)
  "Serialize GAME-STATE to a JSON string."
  (json-encode-plist (state-to-plist state)))

(defun deserialize-state (json-string)
  "Deserialize a JSON string back into a GAME-STATE.
Converts the parsed JSON alist to a plist for internal use."
  (let ((alist (json-decode json-string)))
    (plist-to-state (alist-to-plist alist :key-transform #'string-keyword))))
