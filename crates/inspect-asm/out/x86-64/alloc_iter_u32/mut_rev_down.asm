inspect_asm::alloc_iter_u32::mut_rev_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_11
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	jne .LBB_14
	mov r15, rsi
	shl r14, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rdx, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB_13
	add rax, 3
	and rax, -4
.LBB_4:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp + 8], rdx
	mov qword ptr [rsp + 16], rdi
	mov qword ptr [rsp + 24], 0
	mov qword ptr [rsp + 32], rcx
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor ecx, ecx
	jmp .LBB_6
.LBB_5:
	mov rax, rcx
	inc rcx
	not rax
	mov dword ptr [rdx + 4*rax], ebp
	mov qword ptr [rsp + 24], rcx
	add r13, 4
	cmp r14, r13
	je .LBB_8
.LBB_6:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 32], rcx
	jne .LBB_5
	mov esi, 1
	mov rdi, r12
	call bump_scope::bump_vec_rev::BumpVecRev<T,A,_,_>::generic_grow_cold
	mov rdx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 24]
	jmp .LBB_5
.LBB_8:
	cmp qword ptr [rsp + 32], 0
	je .LBB_11
	shl rax, 2
	add rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	mov rdx, qword ptr [rdx]
	mov qword ptr [rdx], rax
	mov rbx, rax
	jmp .LBB_12
.LBB_11:
	xor ecx, ecx
.LBB_12:
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
.LBB_13:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_greedy_in_another_chunk
	mov rdi, r12
	jmp .LBB_4
.LBB_14:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]