#!/usr/bin/env python3
"""Emit a domain with K procedural craft actions (domain-complexity stress).
Usage: gen_domain.py K > domain.pddl"""
import sys
K=int(sys.argv[1])
p=[]
p.append("(define (domain bigcraft) (:requirements :typing :durative-actions :numeric-fluents)")
p.append("  (:types agent location - object)")
p.append("  (:predicates (at ?a - agent ?l - location) (has-bench ?l - location))")
p.append("  (:functions (dist ?a ?b - location) "+" ".join(f"(raw{i}) (item{i})" for i in range(K))+")")
for i in range(K):
    p.append(f"  (:durative-action craft{i} :parameters (?a - agent ?l - location)")
    p.append(f"    :duration (= ?duration 2)")
    p.append(f"    :condition (and (at start (at ?a ?l)) (at start (has-bench ?l)) (at start (>= (raw{i}) 1)))")
    p.append(f"    :effect (and (at start (decrease (raw{i}) 1)) (at end (increase (item{i}) 1))))")
p.append(")")
print("\n".join(p))
