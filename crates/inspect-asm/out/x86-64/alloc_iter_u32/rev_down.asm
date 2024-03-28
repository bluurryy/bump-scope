inspect_asm::alloc_iter_u32::rev_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_10
	mov r14, rdx
	mov r15, rsi
	mov r12, rdi
	shl r14, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rdx, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB_12
	add rax, 3
	and rax, -4
.LBB_3:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp + 8], rdx
	mov qword ptr [rsp + 16], r12
	mov qword ptr [rsp + 24], 0
	mov qword ptr [rsp + 32], rcx
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor ecx, ecx
	jmp .LBB_5
.LBB_4:
	mov rax, rcx
	not rax
	mov dword ptr [rdx + 4*rax], ebp
	inc rcx
	mov qword ptr [rsp + 24], rcx
	add r13, 4
	cmp r14, r13
	je .LBB_7
.LBB_5:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 32], rcx
	jne .LBB_4
	mov esi, 1
	mov rdi, r12
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,_,_,A>::generic_grow_cold
	mov rdx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 24]
	jmp .LBB_4
.LBB_7:
	cmp qword ptr [rsp + 32], 0
	je .LBB_10
	mov rdx, qword ptr [rsp + 16]
	mov rdx, qword ptr [rdx]
	shl rax, 2
	add rax, qword ptr [rsp + 8]
	mov qword ptr [rdx], rax
	mov rbx, rax
	jmp .LBB_11
.LBB_10:
	xor ecx, ecx
.LBB_11:
	mov rax, rbx
	mov rdx, rcx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_12:
	mov esi, 4
	mov rdi, r12
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<_,_,A>::alloc_greedy_in_another_chunk
	jmp .LBB_3