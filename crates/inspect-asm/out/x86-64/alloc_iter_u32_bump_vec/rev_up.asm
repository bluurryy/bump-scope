inspect_asm::alloc_iter_u32_bump_vec::rev_up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov qword ptr [rsp], 4
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 16], xmm0
	mov qword ptr [rsp + 8], rdi
	test rdx, rdx
	jne .LBB_2
	xor edx, edx
.LBB_6:
	mov r8, qword ptr [rsp + 24]
	test r8, r8
	je .LBB_7
	mov rbx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp]
	lea rax, [4*rdx]
	mov rsi, rcx
	sub rsi, rax
	shl r8, 2
	mov rdi, rcx
	sub rdi, r8
	lea r14, [rdi + 4*rdx]
	cmp r14, rsi
	jbe .LBB_10
	mov rax, rsi
	mov r14, rcx
	jmp .LBB_11
.LBB_7:
	mov eax, 4
	xor edx, edx
	jmp .LBB_12
.LBB_10:
	mov r15, rdx
	mov rdx, rax
	mov r12, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r12
	mov rdx, r15
.LBB_11:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], r14
.LBB_12:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB_2:
	mov rbx, rdx
	mov r14, rsp
	mov rdi, r14
	mov r15, rsi
	mov rsi, rdx
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_>::generic_grow_cold
	mov rax, r15
	mov rcx, qword ptr [rsp + 16]
	shl rbx, 2
	xor r12d, r12d
	jmp .LBB_3
.LBB_5:
	mov rsi, qword ptr [rsp]
	lea rdx, [rcx + 1]
	not rcx
	mov dword ptr [rsi + 4*rcx], ebp
	mov qword ptr [rsp + 16], rdx
	add r12, 4
	mov rcx, rdx
	cmp rbx, r12
	je .LBB_6
.LBB_3:
	mov ebp, dword ptr [rax + r12]
	cmp qword ptr [rsp + 24], rcx
	jne .LBB_5
	mov esi, 1
	mov rdi, r14
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_>::generic_grow_cold
	mov rax, r15
	mov rcx, qword ptr [rsp + 16]
	jmp .LBB_5