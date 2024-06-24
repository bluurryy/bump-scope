inspect_asm::alloc_iter_u32::try_mut_rev_down_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB0_6
	mov rbx, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
.LBB0_0:
	xor eax, eax
	jmp .LBB0_7
.LBB0_1:
	mov r14, rsi
	shl rbx, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, rbx
	jb .LBB0_8
	add rax, 3
	and rax, -4
	je .LBB0_8
.LBB0_2:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp], rdx
	mov qword ptr [rsp + 8], rdi
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rcx
	xor r12d, r12d
	mov r15, rsp
	xor ecx, ecx
	jmp .LBB0_4
.LBB0_3:
	mov rax, rcx
	not rax
	mov dword ptr [rdx + 4*rax], ebp
	inc rcx
	mov qword ptr [rsp + 16], rcx
	add r12, 4
	cmp rbx, r12
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r14 + r12]
	cmp qword ptr [rsp + 24], rcx
	jne .LBB0_3
	mov rdi, r15
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rdx, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 16]
	jmp .LBB0_3
.LBB0_5:
	cmp qword ptr [rsp + 24], 0
	je .LBB0_6
	mov rdx, qword ptr [rsp + 8]
	shl rax, 2
	add rax, qword ptr [rsp]
	mov rdx, qword ptr [rdx]
	mov qword ptr [rdx], rax
	jmp .LBB0_7
.LBB0_6:
	mov eax, 4
	xor ecx, ecx
.LBB0_7:
	mov rdx, rcx
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_8:
	mov esi, 4
	mov r15, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB0_0
	mov rdi, r15
	jmp .LBB0_2
